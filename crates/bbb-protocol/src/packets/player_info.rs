use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::codec::{Decoder, Encoder, ProtocolError, Result};
use crate::component::decode_rich_text_component_from_decoder_with_registry;
use crate::content_registry::ContentRegistry;
pub use crate::game_types::GameType;
use crate::nbt_summary::{encode_nbt_summary_bounded, MAX_NBT_DEPTH};
pub use crate::player_info_actions::PlayerInfoAction;
use crate::RichTextComponent;

use super::{decode_byte_array, GameProfile, GameProfileProperty};

const MAX_PLAYER_INFO_ENTRIES: usize = 8192;
// Matches the component budget `encode_play_clientbound_system_chat` applies.
const DISPLAY_NAME_NBT_BYTES: usize = crate::nbt_summary::VANILLA_DEFAULT_NBT_QUOTA as usize;
const MAX_GAME_PROFILE_PROPERTIES: usize = 1024;
const MAX_PROFILE_PUBLIC_KEY_BYTES: usize = 512;
const MAX_PROFILE_PUBLIC_KEY_SIGNATURE_BYTES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerInfoRemove {
    pub profile_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerInfoUpdate {
    pub actions: Vec<PlayerInfoAction>,
    pub entries: Vec<PlayerInfoEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerInfoEntry {
    pub profile_id: Uuid,
    pub profile: Option<GameProfile>,
    pub listed: bool,
    pub latency: i32,
    pub game_mode: GameType,
    pub display_name: Option<RichTextComponent>,
    pub show_hat: bool,
    pub list_order: i32,
    pub chat_session: Option<PlayerInfoChatSession>,
}

impl PlayerInfoEntry {
    fn new(profile_id: Uuid) -> Self {
        Self {
            profile_id,
            profile: None,
            listed: false,
            latency: 0,
            game_mode: GameType::default(),
            display_name: None,
            show_hat: false,
            list_order: 0,
            chat_session: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerInfoChatSession {
    pub session_id: Uuid,
    pub expires_at_epoch_millis: i64,
    pub public_key: Vec<u8>,
    pub key_signature: Vec<u8>,
}

pub(super) fn decode_player_info_remove(decoder: &mut Decoder<'_>) -> Result<PlayerInfoRemove> {
    let count = decoder.read_len()?;
    if count > MAX_PLAYER_INFO_ENTRIES {
        return Err(ProtocolError::PacketTooLarge(
            count,
            MAX_PLAYER_INFO_ENTRIES,
        ));
    }

    let mut profile_ids = Vec::with_capacity(decoder.bounded_capacity(count, 16));
    for _ in 0..count {
        profile_ids.push(decoder.read_uuid()?);
    }
    let packet = PlayerInfoRemove { profile_ids };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after player info remove packet".to_string(),
        ));
    }
    Ok(packet)
}

pub(super) fn decode_player_info_update_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<PlayerInfoUpdate> {
    let actions = decode_player_info_actions(decoder)?;
    let count = decoder.read_len()?;
    if count > MAX_PLAYER_INFO_ENTRIES {
        return Err(ProtocolError::PacketTooLarge(
            count,
            MAX_PLAYER_INFO_ENTRIES,
        ));
    }

    let mut entries = Vec::with_capacity(decoder.bounded_capacity(count, 16));
    for _ in 0..count {
        let profile_id = decoder.read_uuid()?;
        let mut entry = PlayerInfoEntry::new(profile_id);

        for action in &actions {
            match action {
                PlayerInfoAction::AddPlayer => {
                    let name = decoder.read_string(16)?;
                    let properties = decode_game_profile_properties(decoder)?;
                    entry.profile = Some(GameProfile {
                        uuid: profile_id,
                        name,
                        properties,
                    });
                }
                PlayerInfoAction::InitializeChat => {
                    entry.chat_session = decode_optional_player_info_chat_session(decoder)?;
                }
                PlayerInfoAction::UpdateGameMode => {
                    entry.game_mode = GameType::from_id(decoder.read_var_i32()?);
                }
                PlayerInfoAction::UpdateListed => {
                    entry.listed = decoder.read_bool()?;
                }
                PlayerInfoAction::UpdateLatency => {
                    entry.latency = decoder.read_var_i32()?;
                }
                PlayerInfoAction::UpdateDisplayName => {
                    entry.display_name = decoder
                        .read_bool()?
                        .then(|| {
                            decode_rich_text_component_from_decoder_with_registry(decoder, registry)
                        })
                        .transpose()?;
                }
                PlayerInfoAction::UpdateListOrder => {
                    entry.list_order = decoder.read_var_i32()?;
                }
                PlayerInfoAction::UpdateHat => {
                    entry.show_hat = decoder.read_bool()?;
                }
            }
        }

        entries.push(entry);
    }

    let packet = PlayerInfoUpdate { actions, entries };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after player info update packet".to_string(),
        ));
    }
    Ok(packet)
}

/// `ClientboundPlayerInfoRemovePacket.write`
/// (`ClientboundPlayerInfoRemovePacket.java:20-22`): the profile ids the
/// receiving client drops from its tab list.
pub fn encode_play_clientbound_player_info_remove(
    update: &PlayerInfoRemove,
) -> Result<(i32, Vec<u8>)> {
    if update.profile_ids.len() > MAX_PLAYER_INFO_ENTRIES {
        return Err(ProtocolError::PacketTooLarge(
            update.profile_ids.len(),
            MAX_PLAYER_INFO_ENTRIES,
        ));
    }
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.profile_ids.len() as i32);
    for &profile_id in &update.profile_ids {
        encoder.write_uuid(profile_id);
    }
    Ok((
        crate::ids::play::CLIENTBOUND_PLAYER_INFO_REMOVE,
        encoder.into_inner(),
    ))
}

/// `ClientboundPlayerInfoUpdatePacket.write`
/// (`ClientboundPlayerInfoUpdatePacket.java:70-79`): the action bit set, then
/// one entry per player. Vanilla holds the actions in an `EnumSet`, so both
/// `writeEnumSet` and the per-entry loop run in ascending ordinal order; the
/// sort below reproduces that regardless of how the caller ordered them. Field
/// layout per action: `Action` (`ClientboundPlayerInfoUpdatePacket.java:107-129`).
pub fn encode_play_clientbound_player_info_update(
    update: &PlayerInfoUpdate,
) -> Result<(i32, Vec<u8>)> {
    if update.entries.len() > MAX_PLAYER_INFO_ENTRIES {
        return Err(ProtocolError::PacketTooLarge(
            update.entries.len(),
            MAX_PLAYER_INFO_ENTRIES,
        ));
    }
    let mut actions = update.actions.clone();
    actions.sort_by_key(|action| action.ordinal());
    actions.dedup();
    let mut encoder = Encoder::new();
    encoder.write_u8(
        actions
            .iter()
            .fold(0u8, |bits, action| bits | (1u8 << action.ordinal())),
    );
    encoder.write_var_i32(update.entries.len() as i32);
    for entry in &update.entries {
        encoder.write_uuid(entry.profile_id);
        for action in &actions {
            encode_player_info_action(&mut encoder, *action, entry)?;
        }
    }
    Ok((
        crate::ids::play::CLIENTBOUND_PLAYER_INFO_UPDATE,
        encoder.into_inner(),
    ))
}

fn encode_player_info_action(
    encoder: &mut Encoder,
    action: PlayerInfoAction,
    entry: &PlayerInfoEntry,
) -> Result<()> {
    match action {
        PlayerInfoAction::AddPlayer => {
            let profile = entry.profile.as_ref().ok_or_else(|| {
                ProtocolError::InvalidData(
                    "player info add-player action needs a game profile".to_string(),
                )
            })?;
            encoder.write_string(&profile.name);
            if profile.properties.len() > MAX_GAME_PROFILE_PROPERTIES {
                return Err(ProtocolError::PacketTooLarge(
                    profile.properties.len(),
                    MAX_GAME_PROFILE_PROPERTIES,
                ));
            }
            encoder.write_var_i32(profile.properties.len() as i32);
            for property in &profile.properties {
                encoder.write_string(&property.name);
                encoder.write_string(&property.value);
                match &property.signature {
                    Some(signature) => {
                        encoder.write_bool(true);
                        encoder.write_string(signature);
                    }
                    None => encoder.write_bool(false),
                }
            }
        }
        PlayerInfoAction::InitializeChat => match &entry.chat_session {
            Some(session) => {
                encoder.write_bool(true);
                encoder.write_uuid(session.session_id);
                encoder.write_i64(session.expires_at_epoch_millis);
                encoder.write_var_i32(session.public_key.len() as i32);
                encoder.write_bytes(&session.public_key);
                encoder.write_var_i32(session.key_signature.len() as i32);
                encoder.write_bytes(&session.key_signature);
            }
            None => encoder.write_bool(false),
        },
        PlayerInfoAction::UpdateGameMode => encoder.write_var_i32(entry.game_mode.id()),
        PlayerInfoAction::UpdateListed => encoder.write_bool(entry.listed),
        PlayerInfoAction::UpdateLatency => encoder.write_var_i32(entry.latency),
        PlayerInfoAction::UpdateDisplayName => match &entry.display_name {
            Some(display_name) => {
                encoder.write_bool(true);
                encoder.write_bytes(&encode_nbt_summary_bounded(
                    Some(display_name.component()),
                    DISPLAY_NAME_NBT_BYTES,
                    MAX_NBT_DEPTH,
                )?);
            }
            None => encoder.write_bool(false),
        },
        PlayerInfoAction::UpdateListOrder => encoder.write_var_i32(entry.list_order),
        PlayerInfoAction::UpdateHat => encoder.write_bool(entry.show_hat),
    }
    Ok(())
}

fn decode_player_info_actions(decoder: &mut Decoder<'_>) -> Result<Vec<PlayerInfoAction>> {
    let bits = decoder.read_u8()?;
    let mut actions = Vec::new();
    for ordinal in 0..8 {
        if bits & (1 << ordinal) != 0 {
            actions.push(PlayerInfoAction::from_ordinal(ordinal).ok_or_else(|| {
                ProtocolError::InvalidData(format!("invalid player info action ordinal {ordinal}"))
            })?);
        }
    }
    Ok(actions)
}

fn decode_game_profile_properties(decoder: &mut Decoder<'_>) -> Result<Vec<GameProfileProperty>> {
    let count = decoder.read_len()?;
    if count > MAX_GAME_PROFILE_PROPERTIES {
        return Err(ProtocolError::PacketTooLarge(
            count,
            MAX_GAME_PROFILE_PROPERTIES,
        ));
    }

    let mut properties = Vec::with_capacity(decoder.bounded_capacity(count, 3));
    for _ in 0..count {
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
    Ok(properties)
}

pub(super) fn decode_player_info_chat_session(
    decoder: &mut Decoder<'_>,
) -> Result<PlayerInfoChatSession> {
    Ok(PlayerInfoChatSession {
        session_id: decoder.read_uuid()?,
        expires_at_epoch_millis: decoder.read_i64()?,
        public_key: decode_byte_array(decoder, MAX_PROFILE_PUBLIC_KEY_BYTES, "profile public key")?,
        key_signature: decode_byte_array(
            decoder,
            MAX_PROFILE_PUBLIC_KEY_SIGNATURE_BYTES,
            "profile public key signature",
        )?,
    })
}

fn decode_optional_player_info_chat_session(
    decoder: &mut Decoder<'_>,
) -> Result<Option<PlayerInfoChatSession>> {
    if !decoder.read_bool()? {
        return Ok(None);
    }
    decode_player_info_chat_session(decoder).map(Some)
}

#[cfg(test)]
mod tests {
    use super::{GameType, PlayerInfoAction, PlayerInfoEntry, PlayerInfoRemove, PlayerInfoUpdate};
    use crate::{
        codec::Encoder,
        ids,
        packets::{decode_play_clientbound, GameProfile, GameProfileProperty, PlayClientbound},
    };
    use uuid::Uuid;

    #[test]
    fn decodes_player_info_remove_uuid_list() {
        let first = Uuid::from_u128(0x11111111_2222_3333_4444_555555555555);
        let second = Uuid::from_u128(0xaaaaaaaa_bbbb_cccc_dddd_eeeeeeeeeeee);
        let mut payload = Encoder::new();
        payload.write_var_i32(2);
        payload.write_uuid(first);
        payload.write_uuid(second);
        let payload = payload.into_inner();

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_PLAYER_INFO_REMOVE, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::PlayerInfoRemove(PlayerInfoRemove {
                profile_ids: vec![first, second],
            })
        );

        let mut trailing = payload;
        trailing.push(0);
        assert!(
            decode_play_clientbound(ids::play::CLIENTBOUND_PLAYER_INFO_REMOVE, &trailing)
                .unwrap_err()
                .to_string()
                .contains("trailing bytes after player info remove packet")
        );
    }

    #[test]
    fn decodes_player_info_update_actions_and_signed_property() {
        let profile_id = Uuid::from_u128(0x12345678_1234_5678_90ab_cdef12345678);
        let actions = vec![
            PlayerInfoAction::AddPlayer,
            PlayerInfoAction::UpdateGameMode,
            PlayerInfoAction::UpdateListed,
            PlayerInfoAction::UpdateLatency,
            PlayerInfoAction::UpdateDisplayName,
            PlayerInfoAction::UpdateListOrder,
            PlayerInfoAction::UpdateHat,
        ];

        let mut payload = Encoder::new();
        payload.write_u8(player_info_actions_bits(&actions));
        payload.write_var_i32(1);
        payload.write_uuid(profile_id);
        payload.write_string("Steve");
        payload.write_var_i32(1);
        payload.write_string("textures");
        payload.write_string("texture-value");
        payload.write_bool(true);
        payload.write_string("texture-signature");
        payload.write_var_i32(GameType::Adventure.id());
        payload.write_bool(true);
        payload.write_var_i32(47);
        payload.write_bool(true);
        payload.write_bytes(&nbt_styled_text_root("Captain Steve"));
        payload.write_var_i32(12);
        payload.write_bool(true);
        let payload = payload.into_inner();

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_PLAYER_INFO_UPDATE, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::PlayerInfoUpdate(PlayerInfoUpdate {
                actions,
                entries: vec![PlayerInfoEntry {
                    profile_id,
                    profile: Some(GameProfile {
                        uuid: profile_id,
                        name: "Steve".to_string(),
                        properties: vec![GameProfileProperty {
                            name: "textures".to_string(),
                            value: "texture-value".to_string(),
                            signature: Some("texture-signature".to_string()),
                        }],
                    }),
                    listed: true,
                    latency: 47,
                    game_mode: GameType::Adventure,
                    display_name: Some(
                        crate::RichTextComponent::try_from_component_nbt(
                            crate::NbtSummaryValue::Compound(vec![
                                crate::NbtSummaryEntry {
                                    name: "text".into(),
                                    value: crate::NbtSummaryValue::String("Captain Steve".into(),),
                                },
                                crate::NbtSummaryEntry {
                                    name: "bold".into(),
                                    value: crate::NbtSummaryValue::Byte(1),
                                },
                            ]),
                        )
                        .unwrap(),
                    ),
                    show_hat: true,
                    list_order: 12,
                    chat_session: None,
                }],
            })
        );

        let mut trailing = payload;
        trailing.push(0);
        assert!(
            decode_play_clientbound(ids::play::CLIENTBOUND_PLAYER_INFO_UPDATE, &trailing)
                .unwrap_err()
                .to_string()
                .contains("trailing bytes after player info update packet")
        );
    }

    #[test]
    fn decodes_player_info_update_chat_session_null() {
        let profile_id = Uuid::from_u128(0x22222222_3333_4444_5555_666666666666);
        let actions = vec![PlayerInfoAction::InitializeChat];
        let mut payload = Encoder::new();
        payload.write_u8(player_info_actions_bits(&actions));
        payload.write_var_i32(1);
        payload.write_uuid(profile_id);
        payload.write_bool(false);

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_PLAYER_INFO_UPDATE,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::PlayerInfoUpdate(PlayerInfoUpdate {
                actions,
                entries: vec![PlayerInfoEntry::new(profile_id)],
            })
        );
    }

    #[test]
    fn player_info_update_round_trips_through_the_client_decoder() {
        let profile_id = Uuid::from_u128(0x0a0b0c0d_0e0f_1011_1213_141516171819);
        let packet = PlayerInfoUpdate {
            actions: vec![
                PlayerInfoAction::AddPlayer,
                PlayerInfoAction::InitializeChat,
                PlayerInfoAction::UpdateGameMode,
                PlayerInfoAction::UpdateListed,
                PlayerInfoAction::UpdateLatency,
                PlayerInfoAction::UpdateDisplayName,
                PlayerInfoAction::UpdateListOrder,
                PlayerInfoAction::UpdateHat,
            ],
            entries: vec![PlayerInfoEntry {
                profile_id,
                profile: Some(GameProfile {
                    uuid: profile_id,
                    name: "alice".to_string(),
                    properties: vec![GameProfileProperty {
                        name: "textures".to_string(),
                        value: "value".to_string(),
                        signature: Some("signature".to_string()),
                    }],
                }),
                listed: true,
                latency: 12,
                game_mode: GameType::Creative,
                display_name: None,
                show_hat: true,
                list_order: 3,
                chat_session: None,
            }],
        };

        let (packet_id, payload) = super::encode_play_clientbound_player_info_update(&packet)
            .expect("the initializing entry encodes");
        assert_eq!(packet_id, ids::play::CLIENTBOUND_PLAYER_INFO_UPDATE);
        assert_eq!(
            decode_play_clientbound(packet_id, &payload).unwrap(),
            PlayClientbound::PlayerInfoUpdate(packet)
        );
    }

    #[test]
    fn player_info_update_encodes_actions_in_ascending_ordinal_order() {
        // Vanilla iterates an EnumSet, so a caller-supplied order must not
        // reach the wire; the decoder always reads ascending ordinals.
        let profile_id = Uuid::from_u128(7);
        let scrambled = PlayerInfoUpdate {
            actions: vec![
                PlayerInfoAction::UpdateHat,
                PlayerInfoAction::UpdateListed,
                PlayerInfoAction::UpdateLatency,
            ],
            entries: vec![PlayerInfoEntry {
                profile_id,
                listed: true,
                latency: 5,
                show_hat: true,
                ..PlayerInfoEntry::new(profile_id)
            }],
        };
        let (packet_id, payload) =
            super::encode_play_clientbound_player_info_update(&scrambled).unwrap();
        let PlayClientbound::PlayerInfoUpdate(decoded) =
            decode_play_clientbound(packet_id, &payload).unwrap()
        else {
            panic!("expected a player info update");
        };
        assert_eq!(
            decoded.actions,
            vec![
                PlayerInfoAction::UpdateListed,
                PlayerInfoAction::UpdateLatency,
                PlayerInfoAction::UpdateHat,
            ]
        );
        assert_eq!(decoded.entries, scrambled.entries);
    }

    #[test]
    fn player_info_remove_round_trips_through_the_client_decoder() {
        let packet = PlayerInfoRemove {
            profile_ids: vec![Uuid::from_u128(1), Uuid::from_u128(2)],
        };
        let (packet_id, payload) =
            super::encode_play_clientbound_player_info_remove(&packet).unwrap();
        assert_eq!(packet_id, ids::play::CLIENTBOUND_PLAYER_INFO_REMOVE);
        assert_eq!(
            decode_play_clientbound(packet_id, &payload).unwrap(),
            PlayClientbound::PlayerInfoRemove(packet)
        );
    }

    #[test]
    fn player_info_update_rejects_an_add_player_action_without_a_profile() {
        let profile_id = Uuid::from_u128(9);
        let packet = PlayerInfoUpdate {
            actions: vec![PlayerInfoAction::AddPlayer],
            entries: vec![PlayerInfoEntry::new(profile_id)],
        };
        assert!(super::encode_play_clientbound_player_info_update(&packet)
            .unwrap_err()
            .to_string()
            .contains("needs a game profile"));
    }

    fn player_info_actions_bits(actions: &[PlayerInfoAction]) -> u8 {
        actions
            .iter()
            .fold(0, |bits, action| bits | (1u8 << action.ordinal()))
    }

    fn nbt_styled_text_root(text: &str) -> Vec<u8> {
        let mut payload = vec![10, 8];
        write_mutf8(&mut payload, "text");
        write_mutf8(&mut payload, text);
        payload.push(1);
        write_mutf8(&mut payload, "bold");
        payload.push(1);
        payload.push(0);
        payload
    }

    fn write_mutf8(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
}
