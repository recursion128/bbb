use serde_json::Value;
use uuid::Uuid;

use crate::{
    codec::{Decoder, Encoder, ProtocolError, Result},
    ids,
    nbt_summary::{encode_nbt_summary_bounded, MAX_NBT_DEPTH},
    resource_pack_response_actions::ResourcePackResponseAction,
    RichTextComponent,
};

use super::{
    client_common::{self, CustomClickAction, CustomPayload, ShowDialog},
    connection::{
        decode_client_information_payload, decode_cookie_response_payload,
        decode_known_packs_bounded, encode_cookie_request_payload,
        encode_custom_report_details_payload, encode_known_packs_payload,
        encode_server_links_payload, encode_store_cookie_payload, encode_transfer_payload,
        ClientInformation, ClientIntent, CookieRequest, CustomReportDetails, GameProfile,
        KnownPack, LoginKey, RegistryDataEntry, ServerLinks, StoreCookie, Transfer,
        UpdateEnabledFeatures, MAX_LOGIN_CUSTOM_QUERY_PAYLOAD, MAX_REGISTRY_DATA_ENTRIES,
    },
    decode_byte_array, normalize_resource_location,
    server_presentation::{self, ResourcePackPop},
    tags::{encode_update_tags, UpdateTags},
    validate_login_username,
};

const MAX_STATUS_JSON_UTF16_UNITS: usize = 32_767;
const MAX_GAME_PROFILE_PROPERTIES: usize = 16;
const MAX_COMPONENT_NBT_BYTES: usize = crate::nbt_summary::VANILLA_DEFAULT_NBT_QUOTA as usize;

/// The only Vanilla 26.1 handshake packet, decoded for a server listener.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientIntention {
    pub protocol_version: i32,
    pub host: String,
    pub port: u16,
    pub intent: ClientIntent,
}

/// Serverbound status packets accepted by `ServerStatusPacketListenerImpl`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusServerbound {
    Request,
    Ping { time: i64 },
}

/// Serverbound login packets defined by Vanilla 26.1 (`GameProtocols.LOGIN`).
///
/// Online encryption and custom query/cookie responses decode into structured
/// variants so the offline-only server listener can answer or reject them
/// explicitly according to `ServerLoginPacketListenerImpl`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginServerbound {
    Hello {
        name: String,
        profile_id: Uuid,
    },
    Key(LoginKey),
    CustomQueryAnswer {
        transaction_id: i32,
        payload: Option<Vec<u8>>,
    },
    Acknowledged,
    CookieResponse {
        key: String,
        payload: Option<Vec<u8>>,
    },
}

/// Serverbound configuration packets defined by Vanilla 26.1 (`GameProtocols.CONFIGURATION`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigurationServerbound {
    ClientInformation(ClientInformation),
    CookieResponse {
        key: String,
        payload: Option<Vec<u8>>,
    },
    CustomPayload(CustomPayload),
    Finish,
    KeepAlive {
        id: i64,
    },
    Pong {
        id: i32,
    },
    ResourcePack {
        id: Uuid,
        action: ResourcePackResponseAction,
    },
    SelectKnownPacks {
        known_packs: Vec<KnownPack>,
    },
    CustomClickAction(CustomClickAction),
    AcceptCodeOfConduct,
}

pub fn decode_client_intention(packet_id: i32, payload: &[u8]) -> Result<ClientIntention> {
    if packet_id != ids::handshake::SERVERBOUND_CLIENT_INTENTION {
        return Err(ProtocolError::UnknownPacket {
            state: "handshake serverbound",
            id: packet_id,
        });
    }

    let mut decoder = Decoder::new(payload);
    let protocol_version = decoder.read_var_i32()?;
    let host = decoder.read_string(255)?;
    let port = decoder.read_u16()?;
    let intent_id = decoder.read_var_i32()?;
    let intent = match intent_id {
        1 => ClientIntent::Status,
        2 => ClientIntent::Login,
        3 => ClientIntent::Transfer,
        _ => {
            return Err(ProtocolError::InvalidData(format!(
                "unknown connection intent {intent_id}"
            )))
        }
    };
    ensure_finished(&decoder, "client intention")?;

    Ok(ClientIntention {
        protocol_version,
        host,
        port,
        intent,
    })
}

pub fn decode_status_serverbound(packet_id: i32, payload: &[u8]) -> Result<StatusServerbound> {
    match packet_id {
        ids::status::SERVERBOUND_STATUS_REQUEST => {
            let decoder = Decoder::new(payload);
            ensure_finished(&decoder, "status request")?;
            Ok(StatusServerbound::Request)
        }
        ids::status::SERVERBOUND_PING_REQUEST => {
            let mut decoder = Decoder::new(payload);
            let time = decoder.read_i64()?;
            ensure_finished(&decoder, "status ping request")?;
            Ok(StatusServerbound::Ping { time })
        }
        id => Err(ProtocolError::UnknownPacket {
            state: "status serverbound",
            id,
        }),
    }
}

pub fn decode_login_serverbound(packet_id: i32, payload: &[u8]) -> Result<LoginServerbound> {
    match packet_id {
        ids::login::SERVERBOUND_HELLO => {
            let mut decoder = Decoder::new(payload);
            let name = decoder.read_string(16)?;
            validate_login_username(&name)?;
            let profile_id = decoder.read_uuid()?;
            ensure_finished(&decoder, "login hello")?;
            Ok(LoginServerbound::Hello { name, profile_id })
        }
        ids::login::SERVERBOUND_KEY => {
            let mut decoder = Decoder::new(payload);
            let secret_max = decoder.remaining_len();
            let encrypted_secret =
                decode_byte_array(&mut decoder, secret_max, "login encrypted secret")?;
            let challenge_max = decoder.remaining_len();
            let encrypted_challenge =
                decode_byte_array(&mut decoder, challenge_max, "login encrypted challenge")?;
            ensure_finished(&decoder, "login key")?;
            Ok(LoginServerbound::Key(LoginKey {
                encrypted_secret,
                encrypted_challenge,
            }))
        }
        ids::login::SERVERBOUND_CUSTOM_QUERY_ANSWER => {
            let mut decoder = Decoder::new(payload);
            let transaction_id = decoder.read_var_i32()?;
            let answer_payload = if decoder.read_bool()? {
                let len = decoder.remaining_len();
                if len > MAX_LOGIN_CUSTOM_QUERY_PAYLOAD {
                    return Err(ProtocolError::PacketTooLarge(
                        len,
                        MAX_LOGIN_CUSTOM_QUERY_PAYLOAD,
                    ));
                }
                Some(
                    decoder
                        .read_exact(len, "login custom query answer payload")?
                        .to_vec(),
                )
            } else {
                None
            };
            ensure_finished(&decoder, "login custom query answer")?;
            Ok(LoginServerbound::CustomQueryAnswer {
                transaction_id,
                payload: answer_payload,
            })
        }
        ids::login::SERVERBOUND_LOGIN_ACKNOWLEDGED => {
            let decoder = Decoder::new(payload);
            ensure_finished(&decoder, "login acknowledgement")?;
            Ok(LoginServerbound::Acknowledged)
        }
        ids::login::SERVERBOUND_COOKIE_RESPONSE => {
            let mut decoder = Decoder::new(payload);
            let (key, cookie_payload) = decode_cookie_response_payload(&mut decoder)?;
            ensure_finished(&decoder, "login cookie response")?;
            Ok(LoginServerbound::CookieResponse {
                key,
                payload: cookie_payload,
            })
        }
        id => Err(ProtocolError::UnknownPacket {
            state: "login serverbound",
            id,
        }),
    }
}

pub fn decode_configuration_serverbound(
    packet_id: i32,
    payload: &[u8],
) -> Result<ConfigurationServerbound> {
    let mut decoder = Decoder::new(payload);
    let packet = match packet_id {
        ids::configuration::SERVERBOUND_CLIENT_INFORMATION => {
            ConfigurationServerbound::ClientInformation(decode_client_information_payload(
                &mut decoder,
            )?)
        }
        ids::configuration::SERVERBOUND_COOKIE_RESPONSE => {
            let (key, cookie_payload) = decode_cookie_response_payload(&mut decoder)?;
            ConfigurationServerbound::CookieResponse {
                key,
                payload: cookie_payload,
            }
        }
        ids::configuration::SERVERBOUND_CUSTOM_PAYLOAD => {
            ConfigurationServerbound::CustomPayload(client_common::decode_custom_payload(
                &mut decoder,
                client_common::MAX_SERVERBOUND_CUSTOM_PAYLOAD,
            )?)
        }
        ids::configuration::SERVERBOUND_FINISH_CONFIGURATION => ConfigurationServerbound::Finish,
        ids::configuration::SERVERBOUND_KEEP_ALIVE => ConfigurationServerbound::KeepAlive {
            id: decoder.read_i64()?,
        },
        ids::configuration::SERVERBOUND_PONG => ConfigurationServerbound::Pong {
            id: decoder.read_i32()?,
        },
        ids::configuration::SERVERBOUND_SELECT_KNOWN_PACKS => {
            ConfigurationServerbound::SelectKnownPacks {
                known_packs: decode_known_packs_bounded(&mut decoder, Some(64))?,
            }
        }
        ids::configuration::SERVERBOUND_RESOURCE_PACK => {
            let id = decoder.read_uuid()?;
            let ordinal = decoder.read_var_i32()?;
            ConfigurationServerbound::ResourcePack {
                id,
                action: ResourcePackResponseAction::from_ordinal(ordinal).ok_or_else(|| {
                    ProtocolError::InvalidData(format!(
                        "invalid resource pack response ordinal {ordinal}"
                    ))
                })?,
            }
        }
        ids::configuration::SERVERBOUND_CUSTOM_CLICK_ACTION => {
            ConfigurationServerbound::CustomClickAction(
                client_common::decode_custom_click_action_payload(&mut decoder)?,
            )
        }
        ids::configuration::SERVERBOUND_ACCEPT_CODE_OF_CONDUCT => {
            ConfigurationServerbound::AcceptCodeOfConduct
        }
        id => {
            return Err(ProtocolError::UnknownPacket {
                state: "configuration serverbound",
                id,
            })
        }
    };
    ensure_finished(&decoder, "configuration serverbound")?;
    Ok(packet)
}

pub fn encode_status_response_json(status: &Value) -> Result<(i32, Vec<u8>)> {
    let json = serde_json::to_string(status)
        .map_err(|error| ProtocolError::InvalidData(format!("invalid server status: {error}")))?;
    validate_utf16_len("server status json", &json, MAX_STATUS_JSON_UTF16_UNITS)?;
    let mut encoder = Encoder::new();
    encoder.write_string(&json);
    Ok((
        ids::status::CLIENTBOUND_STATUS_RESPONSE,
        encoder.into_inner(),
    ))
}

pub fn encode_status_pong(time: i64) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_i64(time);
    (ids::status::CLIENTBOUND_PONG_RESPONSE, encoder.into_inner())
}

pub fn encode_login_disconnect_json(reason: &Value) -> Result<(i32, Vec<u8>)> {
    let json = serde_json::to_string(reason)
        .map_err(|error| ProtocolError::InvalidData(format!("invalid login reason: {error}")))?;
    validate_utf16_len("login disconnect component", &json, 262_144)?;
    let mut encoder = Encoder::new();
    encoder.write_string(&json);
    Ok((
        ids::login::CLIENTBOUND_LOGIN_DISCONNECT,
        encoder.into_inner(),
    ))
}

pub fn encode_login_compression(threshold: i32) -> Result<(i32, Vec<u8>)> {
    if threshold < 0 {
        return Err(ProtocolError::InvalidData(format!(
            "login compression threshold must be non-negative, got {threshold}"
        )));
    }
    let mut encoder = Encoder::new();
    encoder.write_var_i32(threshold);
    Ok((
        ids::login::CLIENTBOUND_LOGIN_COMPRESSION,
        encoder.into_inner(),
    ))
}

pub fn encode_login_finished(profile: &GameProfile) -> Result<(i32, Vec<u8>)> {
    validate_login_username(&profile.name)?;
    if profile.properties.len() > MAX_GAME_PROFILE_PROPERTIES {
        return Err(ProtocolError::PacketTooLarge(
            profile.properties.len(),
            MAX_GAME_PROFILE_PROPERTIES,
        ));
    }

    let mut encoder = Encoder::new();
    encoder.write_uuid(profile.uuid);
    encoder.write_string(&profile.name);
    encoder.write_var_i32(profile.properties.len() as i32);
    for property in &profile.properties {
        validate_utf16_len("game profile property name", &property.name, 64)?;
        validate_utf16_len("game profile property value", &property.value, 32_767)?;
        encoder.write_string(&property.name);
        encoder.write_string(&property.value);
        encoder.write_bool(property.signature.is_some());
        if let Some(signature) = &property.signature {
            validate_utf16_len("game profile property signature", signature, 1024)?;
            encoder.write_string(signature);
        }
    }
    Ok((ids::login::CLIENTBOUND_LOGIN_FINISHED, encoder.into_inner()))
}

/// `MinecraftServer.getServerModName` for this server: the configuration
/// brand and the name appended to `level.dat` `ServerBrands`.
pub const BBB_SERVER_BRAND: &str = "bbb-server";
const MAX_CONFIGURATION_CUSTOM_PAYLOAD: usize = 1024 * 1024;

pub fn encode_configuration_brand(brand: &str) -> Result<(i32, Vec<u8>)> {
    validate_utf16_len("server brand", brand, 32_767)?;
    let mut encoder = Encoder::new();
    encoder.write_string("minecraft:brand");
    encoder.write_string(brand);
    Ok((
        ids::configuration::CLIENTBOUND_CUSTOM_PAYLOAD,
        encoder.into_inner(),
    ))
}

pub fn encode_configuration_custom_payload(
    channel: &str,
    raw_payload: &[u8],
) -> Result<(i32, Vec<u8>)> {
    Ok((
        ids::configuration::CLIENTBOUND_CUSTOM_PAYLOAD,
        client_common::encode_custom_payload_bytes(
            channel,
            raw_payload,
            MAX_CONFIGURATION_CUSTOM_PAYLOAD,
        )?,
    ))
}

pub fn encode_configuration_enabled_features(
    update: &UpdateEnabledFeatures,
) -> Result<(i32, Vec<u8>)> {
    let mut features = update
        .features
        .iter()
        .cloned()
        .map(normalize_resource_location)
        .collect::<Result<Vec<_>>>()?;
    features.sort();
    features.dedup();
    let mut encoder = Encoder::new();
    encoder.write_var_i32(features.len() as i32);
    for feature in features {
        encoder.write_string(&feature);
    }
    Ok((
        ids::configuration::CLIENTBOUND_UPDATE_ENABLED_FEATURES,
        encoder.into_inner(),
    ))
}

pub fn encode_configuration_select_known_packs(
    known_packs: &[KnownPack],
) -> Result<(i32, Vec<u8>)> {
    Ok((
        ids::configuration::CLIENTBOUND_SELECT_KNOWN_PACKS,
        encode_known_packs_payload(known_packs, None)?,
    ))
}

/// One synchronized dynamic registry as this server publishes it.
///
/// `RegistrySynchronization.packRegistries` hands its consumer one registry key
/// and that registry's packed elements at a time
/// (`RegistrySynchronization.java:27-35`), and each pair becomes one
/// `ClientboundRegistryDataPacket` (`SynchronizeRegistriesTask.java:35-44`).
/// This is that pair, held by the producer until the configuration phase sends
/// it.
///
/// The element order is the contract, not a detail: `CommonPlayerSpawnInfo`
/// writes its dimension type as a bare registry index
/// (`CommonPlayerSpawnInfo.java:43`, `DimensionType.java:57`,
/// `ByteBufCodecs.java:560-586`), and the client rebuilds that index by
/// registering the entries in the order they arrive
/// (`RegistryLoadTask.java:62-71`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedRegistry {
    pub registry: String,
    pub entries: Vec<RegistryDataEntry>,
}

/// `ClientboundRegistryDataPacket.STREAM_CODEC`
/// (`ClientboundRegistryDataPacket.java:17-25`): the registry key, then the
/// packed elements, each an identifier followed by an optional network NBT tag
/// (`RegistrySynchronization.java:92-98`).
///
/// An entry's payload travels as the raw bytes the caller already holds,
/// because it is the element codec's own output and this layer is not the one
/// that decides what a dimension type or a biome is.
pub fn encode_configuration_registry_data(
    registry: &str,
    entries: &[RegistryDataEntry],
) -> Result<(i32, Vec<u8>)> {
    if entries.len() > MAX_REGISTRY_DATA_ENTRIES {
        return Err(ProtocolError::PacketTooLarge(
            entries.len(),
            MAX_REGISTRY_DATA_ENTRIES,
        ));
    }
    let mut encoder = Encoder::new();
    encoder.write_string(&normalize_resource_location(registry.to_string())?);
    encoder.write_var_i32(entries.len() as i32);
    for entry in entries {
        encoder.write_string(&normalize_resource_location(entry.id.clone())?);
        match &entry.raw_data {
            Some(raw_data) => {
                if raw_data.is_empty() {
                    return Err(ProtocolError::InvalidData(format!(
                        "registry {registry} entry {} carries an empty nbt payload",
                        entry.id
                    )));
                }
                encoder.write_bool(true);
                encoder.write_bytes(raw_data);
            }
            None => encoder.write_bool(false),
        }
    }
    Ok((
        ids::configuration::CLIENTBOUND_REGISTRY_DATA,
        encoder.into_inner(),
    ))
}

pub fn encode_configuration_update_tags(update_tags: &UpdateTags) -> Result<(i32, Vec<u8>)> {
    Ok((
        ids::configuration::CLIENTBOUND_UPDATE_TAGS,
        encode_update_tags(update_tags)?,
    ))
}

pub fn encode_configuration_empty_tags() -> (i32, Vec<u8>) {
    encode_configuration_update_tags(&UpdateTags {
        registries: Vec::new(),
    })
    .expect("an empty update-tags packet is always valid")
}

/// `ClientboundServerLinksPacket` (`ServerLinks.UNTRUSTED_LINKS_STREAM_CODEC`):
/// each entry is `Either<KnownLinkType, Component>` and the link text.
pub fn encode_configuration_server_links(links: &ServerLinks) -> Result<(i32, Vec<u8>)> {
    Ok((
        ids::configuration::CLIENTBOUND_SERVER_LINKS,
        encode_server_links_payload(links)?,
    ))
}

pub fn encode_configuration_cookie_request(request: &CookieRequest) -> Result<(i32, Vec<u8>)> {
    Ok((
        ids::configuration::CLIENTBOUND_COOKIE_REQUEST,
        encode_cookie_request_payload(request)?,
    ))
}

pub fn encode_configuration_ping(id: i32) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_i32(id);
    (ids::configuration::CLIENTBOUND_PING, encoder.into_inner())
}

pub fn encode_configuration_reset_chat() -> (i32, Vec<u8>) {
    (ids::configuration::CLIENTBOUND_RESET_CHAT, Vec::new())
}

pub fn encode_configuration_resource_pack_pop(pop: &ResourcePackPop) -> (i32, Vec<u8>) {
    (
        ids::configuration::CLIENTBOUND_RESOURCE_PACK_POP,
        server_presentation::encode_resource_pack_pop_payload(pop),
    )
}

pub fn encode_configuration_store_cookie(cookie: &StoreCookie) -> Result<(i32, Vec<u8>)> {
    Ok((
        ids::configuration::CLIENTBOUND_STORE_COOKIE,
        encode_store_cookie_payload(cookie)?,
    ))
}

pub fn encode_configuration_transfer(transfer: &Transfer) -> Result<(i32, Vec<u8>)> {
    Ok((
        ids::configuration::CLIENTBOUND_TRANSFER,
        encode_transfer_payload(transfer)?,
    ))
}

pub fn encode_configuration_custom_report_details(
    details: &CustomReportDetails,
) -> Result<(i32, Vec<u8>)> {
    Ok((
        ids::configuration::CLIENTBOUND_CUSTOM_REPORT_DETAILS,
        encode_custom_report_details_payload(details)?,
    ))
}

pub fn encode_configuration_clear_dialog() -> (i32, Vec<u8>) {
    (ids::configuration::CLIENTBOUND_CLEAR_DIALOG, Vec::new())
}

pub fn encode_configuration_show_dialog(dialog: &ShowDialog) -> Result<(i32, Vec<u8>)> {
    Ok((
        ids::configuration::CLIENTBOUND_SHOW_DIALOG,
        client_common::encode_context_free_show_dialog_payload(dialog)?,
    ))
}

/// `ClientboundDisconnectPacket` in the configuration phase
/// (`ComponentSerialization.TRUSTED_CONTEXT_FREE_STREAM_CODEC`).
pub fn encode_configuration_disconnect(reason: &RichTextComponent) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    encoder.write_bytes(&encode_nbt_summary_bounded(
        Some(reason.component()),
        MAX_COMPONENT_NBT_BYTES,
        MAX_NBT_DEPTH,
    )?);
    Ok((
        ids::configuration::CLIENTBOUND_DISCONNECT,
        encoder.into_inner(),
    ))
}

/// `ClientboundCodeOfConductPacket` (`ByteBufCodecs.STRING_UTF8`).
pub fn encode_configuration_code_of_conduct(text: &str) -> Result<(i32, Vec<u8>)> {
    validate_utf16_len("code of conduct", text, 32_767)?;
    let mut encoder = Encoder::new();
    encoder.write_string(text);
    Ok((
        ids::configuration::CLIENTBOUND_CODE_OF_CONDUCT,
        encoder.into_inner(),
    ))
}

/// `ClientboundResourcePackPushPacket.STREAM_CODEC`
/// (`ClientboundResourcePackPushPacket.java:14-24`).
pub fn encode_configuration_resource_pack_push(
    id: Uuid,
    url: &str,
    hash: &str,
    required: bool,
    prompt: Option<&RichTextComponent>,
) -> Result<(i32, Vec<u8>)> {
    validate_utf16_len("resource pack url", url, 32_767)?;
    validate_utf16_len("resource pack hash", hash, 40)?;
    let mut encoder = Encoder::new();
    encoder.write_uuid(id);
    encoder.write_string(url);
    encoder.write_string(hash);
    encoder.write_bool(required);
    encoder.write_bool(prompt.is_some());
    if let Some(prompt) = prompt {
        encoder.write_bytes(&encode_nbt_summary_bounded(
            Some(prompt.component()),
            MAX_COMPONENT_NBT_BYTES,
            MAX_NBT_DEPTH,
        )?);
    }
    Ok((
        ids::configuration::CLIENTBOUND_RESOURCE_PACK_PUSH,
        encoder.into_inner(),
    ))
}

pub fn encode_configuration_keep_alive_clientbound(id: i64) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_i64(id);
    (
        ids::configuration::CLIENTBOUND_KEEP_ALIVE,
        encoder.into_inner(),
    )
}

pub fn encode_configuration_finish_clientbound() -> (i32, Vec<u8>) {
    (
        ids::configuration::CLIENTBOUND_FINISH_CONFIGURATION,
        Vec::new(),
    )
}

fn validate_utf16_len(what: &'static str, value: &str, max: usize) -> Result<()> {
    let actual = value.encode_utf16().count();
    if actual > max {
        return Err(ProtocolError::InvalidData(format!(
            "{what} has {actual} UTF-16 code units, maximum is {max}"
        )));
    }
    Ok(())
}

fn ensure_finished(decoder: &Decoder<'_>, packet: &'static str) -> Result<()> {
    if decoder.is_empty() {
        Ok(())
    } else {
        Err(ProtocolError::InvalidData(format!(
            "trailing bytes after {packet} packet"
        )))
    }
}

#[cfg(test)]
#[path = "server_connection/tests.rs"]
mod tests;
