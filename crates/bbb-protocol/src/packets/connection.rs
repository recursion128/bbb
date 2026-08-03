use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use crate::client_information_options::{
    ClientChatVisibility, ClientMainHand, ClientParticleStatus,
};

use super::{
    client_common::{self, CustomPayload, ShowDialog},
    decode_byte_array, normalize_resource_location, read_resource_location,
    server_presentation::{self, ResourcePackPop, ResourcePackPush},
    tags::UpdateTags,
};

use crate::{
    codec::{Decoder, Encoder, ProtocolError, Result},
    component::{
        decode_component_summary, decode_component_summary_from_decoder,
        decode_json_component_summary, decode_styled_component_summary,
        decode_styled_json_component_summary, StyledTextRun,
    },
    ids, PROTOCOL_VERSION,
};

const MAX_COOKIE_PAYLOAD_SIZE: usize = 5120;
const MAX_GAME_PROFILE_PROPERTIES: usize = 16;
const MAX_LOGIN_CUSTOM_QUERY_PAYLOAD: usize = 1_048_576;
const MAX_SELECTED_KNOWN_PACKS: usize = 64;
const MAX_KNOWN_PACK_STRING_CHARS: usize = 32_767;
const MAX_CUSTOM_REPORT_DETAILS: usize = 32;
const MAX_REGISTRY_DATA_ENTRIES: usize = 131_072;
const MAX_REGISTRY_DATA_NBT_BYTES: usize = 2 * 1024 * 1024;
const MAX_REGISTRY_DATA_NBT_DEPTH: usize = 512;
const MAX_REGISTRY_DATA_NBT_LIST_ITEMS: usize = 1_000_000;

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
pub struct ClientInformation {
    pub language: String,
    pub view_distance: i8,
    pub chat_visibility: ClientChatVisibility,
    pub chat_colors: bool,
    pub displayed_skin_parts: u8,
    pub main_hand: ClientMainHand,
    pub text_filtering_enabled: bool,
    pub allows_listing: bool,
    pub particle_status: ClientParticleStatus,
}

impl Default for ClientInformation {
    fn default() -> Self {
        Self {
            language: "en_us".to_string(),
            view_distance: 10,
            chat_visibility: ClientChatVisibility::Full,
            chat_colors: true,
            displayed_skin_parts: 0x7f,
            main_hand: ClientMainHand::Right,
            text_filtering_enabled: false,
            allows_listing: false,
            particle_status: ClientParticleStatus::All,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoginClientbound {
    Disconnect {
        reason: String,
        styled_reason: Vec<StyledTextRun>,
        raw_reason: String,
    },
    EncryptionRequest(EncryptionRequest),
    LoginFinished {
        profile: GameProfile,
    },
    SetCompression {
        threshold: i32,
    },
    CustomQuery {
        transaction_id: i32,
    },
    CookieRequest(CookieRequest),
    Unknown {
        packet_id: i32,
        len: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptionRequest {
    pub server_id: String,
    pub public_key: Vec<u8>,
    pub challenge: Vec<u8>,
    pub should_authenticate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginKey {
    pub encrypted_secret: Vec<u8>,
    pub encrypted_challenge: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigurationClientbound {
    Disconnect {
        reason: String,
        styled_reason: Vec<StyledTextRun>,
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
    RegistryData(RegistryData),
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
pub struct RegistryData {
    pub registry: String,
    pub entries: Vec<RegistryDataEntry>,
    pub raw_payload_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryDataEntry {
    pub id: String,
    pub raw_data: Option<Vec<u8>>,
}

impl RegistryDataEntry {
    pub fn has_data(&self) -> bool {
        self.raw_data.is_some()
    }

    pub fn raw_data_len(&self) -> usize {
        self.raw_data.as_ref().map_or(0, Vec::len)
    }
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

pub fn validate_handshake_host(host: &str) -> Result<()> {
    let utf16_len = host.encode_utf16().count();
    if utf16_len > 255 {
        return Err(ProtocolError::InvalidData(format!(
            "handshake host has {utf16_len} UTF-16 code units, maximum is 255"
        )));
    }
    Ok(())
}

pub fn encode_handshake(host: &str, port: u16, intent: ClientIntent) -> Result<(i32, Vec<u8>)> {
    validate_handshake_host(host)?;
    let mut out = Encoder::new();
    out.write_var_i32(PROTOCOL_VERSION);
    out.write_string(host);
    out.write_u16(port);
    out.write_var_i32(intent as i32);
    Ok((
        ids::handshake::SERVERBOUND_CLIENT_INTENTION,
        out.into_inner(),
    ))
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

pub fn validate_login_username(username: &str) -> Result<()> {
    let utf16_len = username.encode_utf16().count();
    if utf16_len > 16 {
        return Err(ProtocolError::InvalidData(format!(
            "login username has {utf16_len} UTF-16 code units, maximum is 16"
        )));
    }
    if let Some(ch) = username
        .chars()
        .find(|ch| (*ch as u32) <= 32 || (*ch as u32) >= 127)
    {
        return Err(ProtocolError::InvalidData(format!(
            "login username contains disallowed character U+{:04X}",
            ch as u32
        )));
    }
    Ok(())
}

pub fn encode_login_hello(username: &str, profile_id: Uuid) -> Result<(i32, Vec<u8>)> {
    validate_login_username(username)?;
    let mut out = Encoder::new();
    out.write_string(username);
    out.write_uuid(profile_id);
    Ok((ids::login::SERVERBOUND_HELLO, out.into_inner()))
}

pub fn encode_login_acknowledged() -> (i32, Vec<u8>) {
    (ids::login::SERVERBOUND_LOGIN_ACKNOWLEDGED, Vec::new())
}

pub fn encode_login_key(packet: &LoginKey) -> Result<(i32, Vec<u8>)> {
    let encrypted_secret_len = i32::try_from(packet.encrypted_secret.len()).map_err(|_| {
        ProtocolError::InvalidData(
            "encrypted login secret does not fit a VarInt length".to_string(),
        )
    })?;
    let encrypted_challenge_len =
        i32::try_from(packet.encrypted_challenge.len()).map_err(|_| {
            ProtocolError::InvalidData(
                "encrypted login challenge does not fit a VarInt length".to_string(),
            )
        })?;
    let mut out = Encoder::new();
    out.write_var_i32(encrypted_secret_len);
    out.write_bytes(&packet.encrypted_secret);
    out.write_var_i32(encrypted_challenge_len);
    out.write_bytes(&packet.encrypted_challenge);
    Ok((ids::login::SERVERBOUND_KEY, out.into_inner()))
}

pub fn decode_login_clientbound(packet_id: i32, payload: &[u8]) -> Result<LoginClientbound> {
    match packet_id {
        ids::login::CLIENTBOUND_LOGIN_DISCONNECT => {
            let mut decoder = Decoder::new(payload);
            let raw_reason = decoder.read_string(262144)?;
            if !decoder.is_empty() {
                return Err(ProtocolError::InvalidData(
                    "trailing bytes after login disconnect packet".to_string(),
                ));
            }
            Ok(LoginClientbound::Disconnect {
                reason: decode_json_component_summary(&raw_reason)?,
                styled_reason: decode_styled_json_component_summary(&raw_reason)?,
                raw_reason,
            })
        }
        ids::login::CLIENTBOUND_HELLO => {
            let mut decoder = Decoder::new(payload);
            let server_id = decoder.read_string(20)?;
            let public_key_max = decoder.remaining_len();
            let public_key =
                decode_byte_array(&mut decoder, public_key_max, "login encryption public key")?;
            let challenge_max = decoder.remaining_len();
            let challenge =
                decode_byte_array(&mut decoder, challenge_max, "login encryption challenge")?;
            let should_authenticate = decoder.read_bool()?;
            if !decoder.is_empty() {
                return Err(ProtocolError::InvalidData(
                    "trailing bytes after login hello packet".to_string(),
                ));
            }
            Ok(LoginClientbound::EncryptionRequest(EncryptionRequest {
                server_id,
                public_key,
                challenge,
                should_authenticate,
            }))
        }
        ids::login::CLIENTBOUND_LOGIN_FINISHED => {
            let mut decoder = Decoder::new(payload);
            Ok(LoginClientbound::LoginFinished {
                profile: decode_game_profile(&mut decoder)?,
            })
        }
        ids::login::CLIENTBOUND_LOGIN_COMPRESSION => Ok(LoginClientbound::SetCompression {
            threshold: Decoder::new(payload).read_var_i32()?,
        }),
        ids::login::CLIENTBOUND_CUSTOM_QUERY => {
            let mut decoder = Decoder::new(payload);
            let transaction_id = decoder.read_var_i32()?;
            read_resource_location(&mut decoder)?;
            let payload_len = decoder.remaining_len();
            if payload_len > MAX_LOGIN_CUSTOM_QUERY_PAYLOAD {
                return Err(ProtocolError::PacketTooLarge(
                    payload_len,
                    MAX_LOGIN_CUSTOM_QUERY_PAYLOAD,
                ));
            }
            decoder.read_exact(payload_len, "login custom query payload")?;
            Ok(LoginClientbound::CustomQuery { transaction_id })
        }
        ids::login::CLIENTBOUND_COOKIE_REQUEST => {
            let mut decoder = Decoder::new(payload);
            Ok(LoginClientbound::CookieRequest(decode_cookie_request(
                &mut decoder,
            )?))
        }
        id => Ok(LoginClientbound::Unknown {
            packet_id: id,
            len: payload.len(),
        }),
    }
}

pub fn encode_login_cookie_response(key: &str, payload: Option<&[u8]>) -> Result<(i32, Vec<u8>)> {
    Ok((
        ids::login::SERVERBOUND_COOKIE_RESPONSE,
        encode_cookie_response_payload(key, payload)?,
    ))
}

pub fn encode_login_custom_query_answer(transaction_id: i32) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(transaction_id);
    out.write_bool(false);
    (
        ids::login::SERVERBOUND_CUSTOM_QUERY_ANSWER,
        out.into_inner(),
    )
}

pub fn encode_client_information_default() -> Result<(i32, Vec<u8>)> {
    encode_client_information(&ClientInformation::default())
}

pub fn encode_client_information(information: &ClientInformation) -> Result<(i32, Vec<u8>)> {
    Ok((
        ids::configuration::SERVERBOUND_CLIENT_INFORMATION,
        encode_client_information_payload(information)?,
    ))
}

pub fn encode_configuration_brand_custom_payload(brand: &str) -> Result<(i32, Vec<u8>)> {
    let brand_utf16_len = brand.encode_utf16().count();
    if brand_utf16_len > 32_767 {
        return Err(ProtocolError::InvalidData(format!(
            "brand has {brand_utf16_len} UTF-16 code units, maximum is 32767"
        )));
    }
    let mut out = Encoder::new();
    out.write_string("minecraft:brand");
    out.write_string(brand);
    Ok((
        ids::configuration::SERVERBOUND_CUSTOM_PAYLOAD,
        out.into_inner(),
    ))
}

pub fn encode_configuration_custom_click_action(
    packet: &client_common::CustomClickAction,
) -> Result<(i32, Vec<u8>)> {
    Ok((
        ids::configuration::SERVERBOUND_CUSTOM_CLICK_ACTION,
        client_common::encode_custom_click_action_payload(packet)?,
    ))
}

pub(super) fn encode_client_information_payload(
    information: &ClientInformation,
) -> Result<Vec<u8>> {
    let language_utf16_len = information.language.encode_utf16().count();
    if language_utf16_len > 16 {
        return Err(ProtocolError::InvalidData(format!(
            "client information language has {language_utf16_len} UTF-16 code units, maximum is 16"
        )));
    }
    let mut out = Encoder::new();
    out.write_string(&information.language);
    out.write_i8(information.view_distance);
    out.write_var_i32(information.chat_visibility.id());
    out.write_bool(information.chat_colors);
    out.write_u8(information.displayed_skin_parts);
    out.write_var_i32(information.main_hand.id());
    out.write_bool(information.text_filtering_enabled);
    out.write_bool(information.allows_listing);
    out.write_var_i32(information.particle_status.id());
    Ok(out.into_inner())
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

pub fn encode_configuration_cookie_response(
    key: &str,
    payload: Option<&[u8]>,
) -> Result<(i32, Vec<u8>)> {
    Ok((
        ids::configuration::SERVERBOUND_COOKIE_RESPONSE,
        encode_cookie_response_payload(key, payload)?,
    ))
}

pub fn encode_configuration_pong(id: i32) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_i32(id);
    (ids::configuration::SERVERBOUND_PONG, out.into_inner())
}

pub fn encode_configuration_accept_code_of_conduct() -> (i32, Vec<u8>) {
    (
        ids::configuration::SERVERBOUND_ACCEPT_CODE_OF_CONDUCT,
        Vec::new(),
    )
}

pub fn encode_select_known_packs(known_packs: &[KnownPack]) -> Result<(i32, Vec<u8>)> {
    if known_packs.len() > MAX_SELECTED_KNOWN_PACKS {
        return Err(ProtocolError::InvalidData(format!(
            "selected known pack count {} exceeds vanilla maximum {MAX_SELECTED_KNOWN_PACKS}",
            known_packs.len()
        )));
    }
    let mut out = Encoder::new();
    out.write_var_i32(known_packs.len() as i32);
    for (index, pack) in known_packs.iter().enumerate() {
        for (field, value) in [
            ("namespace", &pack.namespace),
            ("id", &pack.id),
            ("version", &pack.version),
        ] {
            let actual = value.encode_utf16().count();
            if actual > MAX_KNOWN_PACK_STRING_CHARS {
                return Err(ProtocolError::InvalidData(format!(
                    "selected known pack {index} {field} has {actual} UTF-16 code units, maximum is {MAX_KNOWN_PACK_STRING_CHARS}"
                )));
            }
            out.write_string(value);
        }
    }
    Ok((
        ids::configuration::SERVERBOUND_SELECT_KNOWN_PACKS,
        out.into_inner(),
    ))
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
        ids::configuration::CLIENTBOUND_DISCONNECT => Ok(ConfigurationClientbound::Disconnect {
            reason: decode_component_summary(payload)?,
            styled_reason: decode_styled_component_summary(payload)?,
            raw_reason: payload.to_vec(),
        }),
        ids::configuration::CLIENTBOUND_FINISH_CONFIGURATION => {
            Ok(ConfigurationClientbound::Finish)
        }
        ids::configuration::CLIENTBOUND_KEEP_ALIVE => {
            let mut decoder = Decoder::new(payload);
            let id = decoder.read_i64()?;
            if !decoder.is_empty() {
                return Err(ProtocolError::InvalidData(
                    "trailing bytes after configuration keep alive packet".to_string(),
                ));
            }
            Ok(ConfigurationClientbound::KeepAlive { id })
        }
        ids::configuration::CLIENTBOUND_PING => {
            let mut decoder = Decoder::new(payload);
            let id = decoder.read_i32()?;
            if !decoder.is_empty() {
                return Err(ProtocolError::InvalidData(
                    "trailing bytes after configuration ping packet".to_string(),
                ));
            }
            Ok(ConfigurationClientbound::Ping { id })
        }
        ids::configuration::CLIENTBOUND_RESET_CHAT => Ok(ConfigurationClientbound::ResetChat),
        ids::configuration::CLIENTBOUND_REGISTRY_DATA => {
            let mut decoder = Decoder::new(payload);
            Ok(ConfigurationClientbound::RegistryData(
                decode_registry_data(&mut decoder, payload.len())?,
            ))
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
    let mut packs = Vec::with_capacity(crate::codec::bounded_collection_initial_capacity(count));
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
    let mut features = Vec::with_capacity(crate::codec::bounded_collection_initial_capacity(count));
    for _ in 0..count {
        features.push(read_resource_location(decoder)?);
    }
    features.sort();
    features.dedup();
    Ok(UpdateEnabledFeatures { features })
}

fn decode_registry_data(decoder: &mut Decoder<'_>, raw_payload_len: usize) -> Result<RegistryData> {
    let registry = read_resource_location(decoder)?;
    let count = decoder.read_len()?;
    if count > MAX_REGISTRY_DATA_ENTRIES {
        return Err(ProtocolError::PacketTooLarge(
            count,
            MAX_REGISTRY_DATA_ENTRIES,
        ));
    }

    let mut entries = Vec::with_capacity(crate::codec::bounded_collection_initial_capacity(count));
    for _ in 0..count {
        let id = read_resource_location(decoder)?;
        let raw_data = if decoder.read_bool()? {
            Some(read_registry_data_nbt(decoder)?)
        } else {
            None
        };
        entries.push(RegistryDataEntry { id, raw_data });
    }

    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after registry data packet".to_string(),
        ));
    }

    Ok(RegistryData {
        registry,
        entries,
        raw_payload_len,
    })
}

fn read_registry_data_nbt(decoder: &mut Decoder<'_>) -> Result<Vec<u8>> {
    let mut raw = Vec::new();
    let tag_id = read_registry_nbt_u8(decoder, &mut raw, "registry data nbt tag")?;
    if tag_id == 0 {
        return Err(ProtocolError::InvalidData(
            "registry data nbt root must not be TAG_End".to_string(),
        ));
    }
    read_registry_nbt_payload(decoder, &mut raw, tag_id, 0)?;
    Ok(raw)
}

fn read_registry_nbt_payload(
    decoder: &mut Decoder<'_>,
    raw: &mut Vec<u8>,
    tag_id: u8,
    depth: usize,
) -> Result<()> {
    if depth > MAX_REGISTRY_DATA_NBT_DEPTH {
        return Err(ProtocolError::InvalidData(
            "registry data nbt exceeded max depth".to_string(),
        ));
    }

    match tag_id {
        1 => {
            read_registry_nbt_bytes(decoder, raw, 1, "nbt byte")?;
        }
        2 => {
            read_registry_nbt_bytes(decoder, raw, 2, "nbt short")?;
        }
        3 | 5 => {
            read_registry_nbt_bytes(decoder, raw, 4, "nbt int/float")?;
        }
        4 | 6 => {
            read_registry_nbt_bytes(decoder, raw, 8, "nbt long/double")?;
        }
        7 => {
            let len = read_registry_nbt_len(decoder, raw)?;
            read_registry_nbt_bytes(decoder, raw, len, "nbt byte array")?;
        }
        8 => read_registry_nbt_string(decoder, raw)?,
        9 => {
            let element_type = read_registry_nbt_u8(decoder, raw, "nbt list element type")?;
            let len = read_registry_nbt_len(decoder, raw)?;
            if len > MAX_REGISTRY_DATA_NBT_LIST_ITEMS {
                return Err(ProtocolError::PacketTooLarge(
                    len,
                    MAX_REGISTRY_DATA_NBT_LIST_ITEMS,
                ));
            }
            if element_type == 0 && len > 0 {
                return Err(ProtocolError::InvalidData(
                    "non-empty registry data nbt list has end tag element type".to_string(),
                ));
            }
            for _ in 0..len {
                read_registry_nbt_payload(decoder, raw, element_type, depth + 1)?;
            }
        }
        10 => loop {
            let nested_type = read_registry_nbt_u8(decoder, raw, "nbt compound tag")?;
            if nested_type == 0 {
                break;
            }
            read_registry_nbt_string(decoder, raw)?;
            read_registry_nbt_payload(decoder, raw, nested_type, depth + 1)?;
        },
        11 => {
            let len = read_registry_nbt_len(decoder, raw)?;
            let byte_len = len.checked_mul(4).ok_or_else(|| {
                ProtocolError::InvalidData("nbt int array length overflow".to_string())
            })?;
            read_registry_nbt_bytes(decoder, raw, byte_len, "nbt int array")?;
        }
        12 => {
            let len = read_registry_nbt_len(decoder, raw)?;
            let byte_len = len.checked_mul(8).ok_or_else(|| {
                ProtocolError::InvalidData("nbt long array length overflow".to_string())
            })?;
            read_registry_nbt_bytes(decoder, raw, byte_len, "nbt long array")?;
        }
        other => {
            return Err(ProtocolError::InvalidData(format!(
                "invalid registry data nbt tag id {other}"
            )))
        }
    }
    Ok(())
}

fn read_registry_nbt_len(decoder: &mut Decoder<'_>, raw: &mut Vec<u8>) -> Result<usize> {
    let bytes = read_registry_nbt_bytes(decoder, raw, 4, "nbt length")?;
    let len = i32::from_be_bytes(bytes.try_into().expect("fixed length"));
    if len < 0 {
        return Err(ProtocolError::NegativeLength(len));
    }
    Ok(len as usize)
}

fn read_registry_nbt_string(decoder: &mut Decoder<'_>, raw: &mut Vec<u8>) -> Result<()> {
    let len_bytes = read_registry_nbt_bytes(decoder, raw, 2, "nbt string length")?;
    let len = u16::from_be_bytes(len_bytes.try_into().expect("fixed length")) as usize;
    read_registry_nbt_bytes(decoder, raw, len, "nbt string")?;
    Ok(())
}

fn read_registry_nbt_u8(
    decoder: &mut Decoder<'_>,
    raw: &mut Vec<u8>,
    what: &'static str,
) -> Result<u8> {
    Ok(read_registry_nbt_bytes(decoder, raw, 1, what)?[0])
}

fn read_registry_nbt_bytes<'a>(
    decoder: &mut Decoder<'a>,
    raw: &mut Vec<u8>,
    len: usize,
    what: &'static str,
) -> Result<&'a [u8]> {
    let next_len = raw
        .len()
        .checked_add(len)
        .ok_or_else(|| ProtocolError::InvalidData("registry nbt length overflow".to_string()))?;
    if next_len > MAX_REGISTRY_DATA_NBT_BYTES {
        return Err(ProtocolError::PacketTooLarge(
            next_len,
            MAX_REGISTRY_DATA_NBT_BYTES,
        ));
    }
    let bytes = decoder.read_exact(len, what)?;
    raw.extend_from_slice(bytes);
    Ok(bytes)
}

pub(super) fn decode_cookie_request(decoder: &mut Decoder<'_>) -> Result<CookieRequest> {
    let request = CookieRequest {
        key: read_resource_location(decoder)?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after cookie request packet".to_string(),
        ));
    }
    Ok(request)
}

pub(super) fn decode_store_cookie(decoder: &mut Decoder<'_>) -> Result<StoreCookie> {
    let key = read_resource_location(decoder)?;
    let len = decoder.read_len()?;
    if len > MAX_COOKIE_PAYLOAD_SIZE {
        return Err(ProtocolError::PacketTooLarge(len, MAX_COOKIE_PAYLOAD_SIZE));
    }
    let cookie = StoreCookie {
        key,
        payload: decoder.read_exact(len, "cookie payload")?.to_vec(),
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after store cookie packet".to_string(),
        ));
    }
    Ok(cookie)
}

pub(super) fn decode_transfer(decoder: &mut Decoder<'_>) -> Result<Transfer> {
    let transfer = Transfer {
        host: decoder.read_string(32767)?,
        port: decoder.read_var_i32()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after transfer packet".to_string(),
        ));
    }
    Ok(transfer)
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
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after custom report details packet".to_string(),
        ));
    }
    Ok(CustomReportDetails { details })
}

pub(super) fn decode_server_links(decoder: &mut Decoder<'_>) -> Result<ServerLinks> {
    let count = decoder.read_len()?;
    let mut links = Vec::with_capacity(crate::codec::bounded_collection_initial_capacity(count));
    for _ in 0..count {
        links.push(decode_server_link_entry(decoder)?);
    }
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after server links packet".to_string(),
        ));
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

pub(super) fn encode_cookie_response_payload(key: &str, payload: Option<&[u8]>) -> Result<Vec<u8>> {
    let key = normalize_resource_location(key.to_string())?;
    if let Some(payload) = payload {
        if payload.len() > MAX_COOKIE_PAYLOAD_SIZE {
            return Err(ProtocolError::PacketTooLarge(
                payload.len(),
                MAX_COOKIE_PAYLOAD_SIZE,
            ));
        }
    }
    let mut out = Encoder::new();
    out.write_string(&key);
    if let Some(payload) = payload {
        out.write_bool(true);
        out.write_var_i32(payload.len() as i32);
        out.write_bytes(payload);
    } else {
        out.write_bool(false);
    }
    Ok(out.into_inner())
}

fn decode_game_profile(decoder: &mut Decoder<'_>) -> Result<GameProfile> {
    let uuid = decoder.read_uuid()?;
    let name = decoder.read_string(16)?;
    let property_count = decoder.read_len()?;
    if property_count > MAX_GAME_PROFILE_PROPERTIES {
        return Err(ProtocolError::PacketTooLarge(
            property_count,
            MAX_GAME_PROFILE_PROPERTIES,
        ));
    }
    let mut properties = Vec::with_capacity(crate::codec::bounded_collection_initial_capacity(
        property_count,
    ));
    for _ in 0..property_count {
        let name = decoder.read_string(64)?;
        let value = decoder.read_string(32767)?;
        let signature = if decoder.read_bool()? {
            Some(decoder.read_string(1024)?)
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
