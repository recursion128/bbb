use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    codec::{Decoder, Encoder, Result},
    component::decode_component_summary_from_decoder,
    ids,
};

use super::{
    decode_optional_byte_array, decode_optional_component_summary_from_decoder,
    decode_optional_uuid, MAX_SERVER_ICON_BYTES,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TabList {
    pub header: Option<String>,
    pub footer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourcePackPush {
    pub id: Uuid,
    pub url: String,
    pub hash: String,
    pub required: bool,
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourcePackPop {
    pub id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerData {
    pub motd: String,
    pub icon_bytes: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourcePackResponseAction {
    SuccessfullyLoaded,
    Declined,
    FailedDownload,
    Accepted,
    Downloaded,
    InvalidUrl,
    FailedReload,
    Discarded,
}

impl ResourcePackResponseAction {
    pub fn ordinal(self) -> i32 {
        match self {
            Self::SuccessfullyLoaded => 0,
            Self::Declined => 1,
            Self::FailedDownload => 2,
            Self::Accepted => 3,
            Self::Downloaded => 4,
            Self::InvalidUrl => 5,
            Self::FailedReload => 6,
            Self::Discarded => 7,
        }
    }
}

pub fn encode_configuration_resource_pack_response(
    id: Uuid,
    action: ResourcePackResponseAction,
) -> (i32, Vec<u8>) {
    (
        ids::configuration::SERVERBOUND_RESOURCE_PACK,
        encode_resource_pack_response_payload(id, action),
    )
}

pub fn encode_play_resource_pack_response(
    id: Uuid,
    action: ResourcePackResponseAction,
) -> (i32, Vec<u8>) {
    (
        ids::play::SERVERBOUND_RESOURCE_PACK,
        encode_resource_pack_response_payload(id, action),
    )
}

fn encode_resource_pack_response_payload(id: Uuid, action: ResourcePackResponseAction) -> Vec<u8> {
    let mut out = Encoder::new();
    out.write_uuid(id);
    out.write_var_i32(action.ordinal());
    out.into_inner()
}

pub(super) fn decode_resource_pack_pop(decoder: &mut Decoder<'_>) -> Result<ResourcePackPop> {
    Ok(ResourcePackPop {
        id: decode_optional_uuid(decoder)?,
    })
}

pub(super) fn decode_resource_pack_push(decoder: &mut Decoder<'_>) -> Result<ResourcePackPush> {
    Ok(ResourcePackPush {
        id: decoder.read_uuid()?,
        url: decoder.read_string(32767)?,
        hash: decoder.read_string(40)?,
        required: decoder.read_bool()?,
        prompt: decode_optional_component_summary_from_decoder(decoder)?,
    })
}

pub(super) fn decode_server_data(decoder: &mut Decoder<'_>) -> Result<ServerData> {
    Ok(ServerData {
        motd: decode_component_summary_from_decoder(decoder)?,
        icon_bytes: decode_optional_byte_array(decoder, MAX_SERVER_ICON_BYTES, "server icon")?,
    })
}

pub(super) fn decode_tab_list(decoder: &mut Decoder<'_>) -> Result<TabList> {
    Ok(TabList {
        header: decode_tab_list_component(decoder)?,
        footer: decode_tab_list_component(decoder)?,
    })
}

fn decode_tab_list_component(decoder: &mut Decoder<'_>) -> Result<Option<String>> {
    let before = decoder.remaining();
    let summary = decode_component_summary_from_decoder(decoder)?;
    let consumed_len = before.len().saturating_sub(decoder.remaining_len());
    let consumed = &before[..consumed_len];

    if is_empty_string_component_nbt(consumed) {
        Ok(None)
    } else {
        Ok(Some(summary))
    }
}

fn is_empty_string_component_nbt(payload: &[u8]) -> bool {
    payload == [8, 0, 0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        codec::{Decoder, Encoder},
        ids,
        packets::{decode_play_clientbound, PlayClientbound},
    };

    #[test]
    fn decodes_tab_list_header_footer() {
        let mut payload = Encoder::new();
        payload.write_bytes(&nbt_string_root("Online players"));
        payload.write_bytes(&nbt_string_root(""));
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_TAB_LIST, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::TabList(TabList {
                header: Some("Online players".to_string()),
                footer: None,
            })
        );

        let mut payload = Encoder::new();
        payload.write_bytes(&nbt_string_root(""));
        payload.write_bytes(&nbt_string_root("Welcome"));
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_TAB_LIST, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::TabList(TabList {
                header: None,
                footer: Some("Welcome".to_string()),
            })
        );
    }

    #[test]
    fn decodes_resource_pack_push_packet() {
        let pack_id = Uuid::from_u128(0x11111111_2222_3333_4444_555555555555);
        let mut payload = Encoder::new();
        payload.write_uuid(pack_id);
        payload.write_string("https://example.invalid/server-pack.zip");
        payload.write_string("0123456789abcdef0123456789abcdef01234567");
        payload.write_bool(true);
        payload.write_bool(true);
        payload.write_bytes(&nbt_string_root("Install this pack"));

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_RESOURCE_PACK_PUSH,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::ResourcePackPush(ResourcePackPush {
                id: pack_id,
                url: "https://example.invalid/server-pack.zip".to_string(),
                hash: "0123456789abcdef0123456789abcdef01234567".to_string(),
                required: true,
                prompt: Some("Install this pack".to_string()),
            })
        );
    }

    #[test]
    fn decodes_resource_pack_pop_packet() {
        let pack_id = Uuid::from_u128(0xaaaaaaaa_bbbb_cccc_dddd_eeeeeeeeeeee);
        let mut payload = Encoder::new();
        payload.write_bool(true);
        payload.write_uuid(pack_id);

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_RESOURCE_PACK_POP,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::ResourcePackPop(ResourcePackPop { id: Some(pack_id) })
        );
    }

    #[test]
    fn decodes_server_data_packet_with_icon() {
        let icon_bytes = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        let mut payload = Encoder::new();
        payload.write_bytes(&nbt_string_root("A native test server"));
        payload.write_bool(true);
        payload.write_var_i32(icon_bytes.len() as i32);
        payload.write_bytes(&icon_bytes);

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SERVER_DATA, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::ServerData(ServerData {
                motd: "A native test server".to_string(),
                icon_bytes: Some(icon_bytes),
            })
        );
    }

    #[test]
    fn encodes_resource_pack_response_packets() {
        let pack_id = Uuid::from_u128(0x22222222_3333_4444_5555_666666666666);
        let expected_ordinals = [
            (ResourcePackResponseAction::SuccessfullyLoaded, 0),
            (ResourcePackResponseAction::Declined, 1),
            (ResourcePackResponseAction::FailedDownload, 2),
            (ResourcePackResponseAction::Accepted, 3),
            (ResourcePackResponseAction::Downloaded, 4),
            (ResourcePackResponseAction::InvalidUrl, 5),
            (ResourcePackResponseAction::FailedReload, 6),
            (ResourcePackResponseAction::Discarded, 7),
        ];
        for (action, expected) in expected_ordinals {
            assert_eq!(action.ordinal(), expected);
        }

        let (id, payload) = encode_configuration_resource_pack_response(
            pack_id,
            ResourcePackResponseAction::Accepted,
        );
        assert_eq!(id, ids::configuration::SERVERBOUND_RESOURCE_PACK);
        assert_resource_pack_response_payload(
            &payload,
            pack_id,
            ResourcePackResponseAction::Accepted,
        );

        let (id, payload) =
            encode_play_resource_pack_response(pack_id, ResourcePackResponseAction::FailedReload);
        assert_eq!(id, ids::play::SERVERBOUND_RESOURCE_PACK);
        assert_eq!(id, 49);
        assert_resource_pack_response_payload(
            &payload,
            pack_id,
            ResourcePackResponseAction::FailedReload,
        );
    }

    fn nbt_string_root(text: &str) -> Vec<u8> {
        let mut payload = vec![8];
        payload.extend_from_slice(&(text.len() as u16).to_be_bytes());
        payload.extend_from_slice(text.as_bytes());
        payload
    }

    fn assert_resource_pack_response_payload(
        payload: &[u8],
        expected_id: Uuid,
        expected_action: ResourcePackResponseAction,
    ) {
        let mut decoder = Decoder::new(payload);
        assert_eq!(decoder.read_uuid().unwrap(), expected_id);
        assert_eq!(decoder.read_var_i32().unwrap(), expected_action.ordinal());
        assert!(decoder.is_empty());
    }
}
