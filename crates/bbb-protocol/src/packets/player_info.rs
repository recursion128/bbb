use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::codec::{Decoder, ProtocolError, Result};

use super::{
    decode_byte_array, decode_optional_component_summary_from_decoder, GameProfile,
    GameProfileProperty,
};

const MAX_PLAYER_INFO_ENTRIES: usize = 8192;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerInfoAction {
    AddPlayer,
    InitializeChat,
    UpdateGameMode,
    UpdateListed,
    UpdateLatency,
    UpdateDisplayName,
    UpdateListOrder,
    UpdateHat,
}

impl PlayerInfoAction {
    pub fn ordinal(self) -> u8 {
        match self {
            Self::AddPlayer => 0,
            Self::InitializeChat => 1,
            Self::UpdateGameMode => 2,
            Self::UpdateListed => 3,
            Self::UpdateLatency => 4,
            Self::UpdateDisplayName => 5,
            Self::UpdateListOrder => 6,
            Self::UpdateHat => 7,
        }
    }

    fn from_ordinal(ordinal: u8) -> Result<Self> {
        Ok(match ordinal {
            0 => Self::AddPlayer,
            1 => Self::InitializeChat,
            2 => Self::UpdateGameMode,
            3 => Self::UpdateListed,
            4 => Self::UpdateLatency,
            5 => Self::UpdateDisplayName,
            6 => Self::UpdateListOrder,
            7 => Self::UpdateHat,
            other => {
                return Err(ProtocolError::InvalidData(format!(
                    "invalid player info action ordinal {other}"
                )));
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerInfoEntry {
    pub profile_id: Uuid,
    pub profile: Option<GameProfile>,
    pub listed: bool,
    pub latency: i32,
    pub game_mode: GameType,
    pub display_name: Option<String>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameType {
    Survival,
    Creative,
    Adventure,
    Spectator,
}

impl Default for GameType {
    fn default() -> Self {
        Self::Survival
    }
}

impl GameType {
    pub fn id(self) -> i32 {
        match self {
            Self::Survival => 0,
            Self::Creative => 1,
            Self::Adventure => 2,
            Self::Spectator => 3,
        }
    }

    pub fn from_id(id: i32) -> Self {
        match id {
            0 => Self::Survival,
            1 => Self::Creative,
            2 => Self::Adventure,
            3 => Self::Spectator,
            _ => Self::Survival,
        }
    }
}

pub(super) fn decode_player_info_remove(decoder: &mut Decoder<'_>) -> Result<PlayerInfoRemove> {
    let count = decoder.read_len()?;
    if count > MAX_PLAYER_INFO_ENTRIES {
        return Err(ProtocolError::PacketTooLarge(
            count,
            MAX_PLAYER_INFO_ENTRIES,
        ));
    }

    let mut profile_ids = Vec::with_capacity(count);
    for _ in 0..count {
        profile_ids.push(decoder.read_uuid()?);
    }
    Ok(PlayerInfoRemove { profile_ids })
}

pub(super) fn decode_player_info_update(decoder: &mut Decoder<'_>) -> Result<PlayerInfoUpdate> {
    let actions = decode_player_info_actions(decoder)?;
    let count = decoder.read_len()?;
    if count > MAX_PLAYER_INFO_ENTRIES {
        return Err(ProtocolError::PacketTooLarge(
            count,
            MAX_PLAYER_INFO_ENTRIES,
        ));
    }

    let mut entries = Vec::with_capacity(count);
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
                    entry.display_name = decode_optional_component_summary_from_decoder(decoder)?;
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

    Ok(PlayerInfoUpdate { actions, entries })
}

fn decode_player_info_actions(decoder: &mut Decoder<'_>) -> Result<Vec<PlayerInfoAction>> {
    let bits = decoder.read_u8()?;
    let mut actions = Vec::new();
    for ordinal in 0..8 {
        if bits & (1 << ordinal) != 0 {
            actions.push(PlayerInfoAction::from_ordinal(ordinal)?);
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

    let mut properties = Vec::with_capacity(count);
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

fn decode_optional_player_info_chat_session(
    decoder: &mut Decoder<'_>,
) -> Result<Option<PlayerInfoChatSession>> {
    if !decoder.read_bool()? {
        return Ok(None);
    }

    Ok(Some(PlayerInfoChatSession {
        session_id: decoder.read_uuid()?,
        expires_at_epoch_millis: decoder.read_i64()?,
        public_key: decode_byte_array(decoder, MAX_PROFILE_PUBLIC_KEY_BYTES, "profile public key")?,
        key_signature: decode_byte_array(
            decoder,
            MAX_PROFILE_PUBLIC_KEY_SIGNATURE_BYTES,
            "profile public key signature",
        )?,
    }))
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

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_PLAYER_INFO_REMOVE,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::PlayerInfoRemove(PlayerInfoRemove {
                profile_ids: vec![first, second],
            })
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
        payload.write_bytes(&nbt_string_root("Captain Steve"));
        payload.write_var_i32(12);
        payload.write_bool(true);

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_PLAYER_INFO_UPDATE,
            &payload.into_inner(),
        )
        .unwrap();
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
                    display_name: Some("Captain Steve".to_string()),
                    show_hat: true,
                    list_order: 12,
                    chat_session: None,
                }],
            })
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

    fn player_info_actions_bits(actions: &[PlayerInfoAction]) -> u8 {
        actions
            .iter()
            .fold(0, |bits, action| bits | (1u8 << action.ordinal()))
    }

    fn nbt_string_root(text: &str) -> Vec<u8> {
        let mut payload = vec![8];
        payload.extend_from_slice(&(text.len() as u16).to_be_bytes());
        payload.extend_from_slice(text.as_bytes());
        payload
    }
}
