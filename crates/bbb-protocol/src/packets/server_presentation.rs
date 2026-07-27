use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use crate::resource_pack_response_actions::ResourcePackResponseAction;
use crate::{
    codec::{Decoder, Encoder, ProtocolError, Result},
    component::{decode_component_summary_from_decoder, decode_rich_text_component_from_decoder},
    ids, RichTextComponent,
};

use super::{
    decode_optional_byte_array, decode_optional_component_summary_from_decoder,
    decode_optional_uuid, MAX_SERVER_ICON_BYTES,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TabList {
    pub header: RichTextComponent,
    pub footer: RichTextComponent,
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
    let packet = ResourcePackPop {
        id: decode_optional_uuid(decoder)?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after resource pack pop packet".to_string(),
        ));
    }
    Ok(packet)
}

pub(super) fn decode_resource_pack_push(decoder: &mut Decoder<'_>) -> Result<ResourcePackPush> {
    let packet = ResourcePackPush {
        id: decoder.read_uuid()?,
        url: decoder.read_string(32767)?,
        hash: decoder.read_string(40)?,
        required: decoder.read_bool()?,
        prompt: decode_optional_component_summary_from_decoder(decoder)?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after resource pack push packet".to_string(),
        ));
    }
    Ok(packet)
}

pub(super) fn decode_server_data(decoder: &mut Decoder<'_>) -> Result<ServerData> {
    let update = ServerData {
        motd: decode_component_summary_from_decoder(decoder)?,
        icon_bytes: decode_optional_byte_array(decoder, MAX_SERVER_ICON_BYTES, "server icon")?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after server data packet".to_string(),
        ));
    }
    Ok(update)
}

pub(super) fn decode_tab_list(decoder: &mut Decoder<'_>) -> Result<TabList> {
    let update = TabList {
        header: decode_rich_text_component_from_decoder(decoder)?,
        footer: decode_rich_text_component_from_decoder(decoder)?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after tab list packet".to_string(),
        ));
    }
    Ok(update)
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
                header: RichTextComponent::plain("Online players"),
                footer: RichTextComponent::plain(""),
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
                header: RichTextComponent::plain(""),
                footer: RichTextComponent::plain("Welcome"),
            })
        );

        let mut payload = Encoder::new();
        payload.write_bytes(&nbt_styled_text_root("Styled", "red"));
        payload.write_bytes(&nbt_string_root(""));
        let PlayClientbound::TabList(packet) =
            decode_play_clientbound(ids::play::CLIENTBOUND_TAB_LIST, &payload.into_inner())
                .unwrap()
        else {
            panic!("expected tab-list packet");
        };
        assert_eq!(packet.header.text(), "Styled");
        assert_eq!(packet.header.styled().len(), 1);
        assert_eq!(packet.header.styled()[0].style.bold, Some(true));
        assert_eq!(packet.header.styled()[0].style.color, Some(0xFF_55_55));
        assert!(matches!(
            packet.header.component(),
            crate::NbtSummaryValue::Compound(_)
        ));
        assert!(packet.footer.is_empty());

        let mut trailing = Encoder::new();
        trailing.write_bytes(&nbt_string_root("Online players"));
        trailing.write_bytes(&nbt_string_root(""));
        trailing.write_u8(0);
        assert!(
            decode_play_clientbound(ids::play::CLIENTBOUND_TAB_LIST, &trailing.into_inner())
                .unwrap_err()
                .to_string()
                .contains("trailing bytes after tab list packet")
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

        let payload = payload.into_inner();
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_RESOURCE_PACK_PUSH, &payload).unwrap();
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

        let mut trailing = payload;
        trailing.push(0);
        assert!(
            decode_play_clientbound(ids::play::CLIENTBOUND_RESOURCE_PACK_PUSH, &trailing)
                .unwrap_err()
                .to_string()
                .contains("trailing bytes after resource pack push packet")
        );
    }

    #[test]
    fn decodes_resource_pack_pop_packet() {
        let pack_id = Uuid::from_u128(0xaaaaaaaa_bbbb_cccc_dddd_eeeeeeeeeeee);
        let mut payload = Encoder::new();
        payload.write_bool(true);
        payload.write_uuid(pack_id);

        let payload = payload.into_inner();
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_RESOURCE_PACK_POP, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::ResourcePackPop(ResourcePackPop { id: Some(pack_id) })
        );

        let mut trailing = payload;
        trailing.push(0);
        assert!(
            decode_play_clientbound(ids::play::CLIENTBOUND_RESOURCE_PACK_POP, &trailing)
                .unwrap_err()
                .to_string()
                .contains("trailing bytes after resource pack pop packet")
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

        let mut payload = Encoder::new();
        payload.write_bytes(&nbt_string_root("No icon"));
        payload.write_bool(false);
        assert_eq!(
            decode_play_clientbound(ids::play::CLIENTBOUND_SERVER_DATA, &payload.into_inner())
                .unwrap(),
            PlayClientbound::ServerData(ServerData {
                motd: "No icon".to_string(),
                icon_bytes: None,
            })
        );

        let mut trailing = Encoder::new();
        trailing.write_bytes(&nbt_string_root("No icon"));
        trailing.write_bool(false);
        trailing.write_u8(0);
        assert!(decode_play_clientbound(
            ids::play::CLIENTBOUND_SERVER_DATA,
            &trailing.into_inner()
        )
        .unwrap_err()
        .to_string()
        .contains("trailing bytes after server data packet"));
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

    fn nbt_styled_text_root(text: &str, color: &str) -> Vec<u8> {
        let mut payload = vec![10, 8];
        write_nbt_string(&mut payload, "text");
        write_nbt_string(&mut payload, text);
        payload.push(1);
        write_nbt_string(&mut payload, "bold");
        payload.push(1);
        payload.push(8);
        write_nbt_string(&mut payload, "color");
        write_nbt_string(&mut payload, color);
        payload.push(0);
        payload
    }

    fn write_nbt_string(payload: &mut Vec<u8>, value: &str) {
        payload.extend_from_slice(&(value.len() as u16).to_be_bytes());
        payload.extend_from_slice(value.as_bytes());
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
