use super::*;
use crate::{
    codec::{Decoder, Encoder},
    ids,
    packets::{decode_play_clientbound, PlayClientbound},
};

#[test]
fn decodes_brand_custom_payload() {
    let mut payload = Encoder::new();
    payload.write_string("minecraft:brand");
    payload.write_string("vanilla");
    let payload = payload.into_inner();

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_CUSTOM_PAYLOAD, &payload).unwrap(),
        PlayClientbound::CustomPayload(CustomPayload {
            id: "minecraft:brand".to_string(),
            payload: CustomPayloadBody::Brand {
                brand: "vanilla".to_string(),
            },
        })
    );

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(32767).unwrap(), "minecraft:brand");
    assert_eq!(decoder.read_string(32767).unwrap(), "vanilla");
    assert!(decoder.is_empty());
}

#[test]
fn decodes_brand_custom_payload_with_default_namespace() {
    let mut payload = Encoder::new();
    payload.write_string("brand");
    payload.write_string("vanilla");
    let payload = payload.into_inner();

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_CUSTOM_PAYLOAD, &payload).unwrap(),
        PlayClientbound::CustomPayload(CustomPayload {
            id: "minecraft:brand".to_string(),
            payload: CustomPayloadBody::Brand {
                brand: "vanilla".to_string(),
            },
        })
    );
}

#[test]
fn normalizes_unknown_custom_payload_id() {
    let mut payload = Encoder::new();
    payload.write_string("debug/path");
    payload.write_bytes(&[0xaa, 0xbb, 0xcc]);
    let payload = payload.into_inner();

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_CUSTOM_PAYLOAD, &payload).unwrap(),
        PlayClientbound::CustomPayload(CustomPayload {
            id: "minecraft:debug/path".to_string(),
            payload: CustomPayloadBody::Unknown {
                raw_payload: vec![0xaa, 0xbb, 0xcc],
            },
        })
    );
}

#[test]
fn rejects_invalid_custom_payload_id() {
    let mut payload = Encoder::new();
    payload.write_string("minecraft:Brand");
    payload.write_string("vanilla");
    let payload = payload.into_inner();

    let err = decode_play_clientbound(ids::play::CLIENTBOUND_CUSTOM_PAYLOAD, &payload).unwrap_err();
    assert!(err.to_string().contains("invalid resource location"));
}

#[test]
fn decodes_unknown_custom_payload_raw_body() {
    let mut payload = Encoder::new();
    payload.write_string("bbb:test");
    payload.write_bytes(&[0xaa, 0xbb, 0xcc]);
    let payload = payload.into_inner();

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_CUSTOM_PAYLOAD, &payload).unwrap(),
        PlayClientbound::CustomPayload(CustomPayload {
            id: "bbb:test".to_string(),
            payload: CustomPayloadBody::Unknown {
                raw_payload: vec![0xaa, 0xbb, 0xcc],
            },
        })
    );
}

#[test]
fn decodes_clear_dialog_as_empty_packet() {
    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_CLEAR_DIALOG, &[]).unwrap(),
        PlayClientbound::ClearDialog
    );
}

#[test]
fn rejects_clear_dialog_with_trailing_bytes() {
    let err = decode_play_clientbound(ids::play::CLIENTBOUND_CLEAR_DIALOG, &[0]).unwrap_err();
    assert!(err
        .to_string()
        .contains("trailing bytes after clear dialog packet"));
}

#[test]
fn decodes_show_dialog_reference_holder() {
    let mut payload = Encoder::new();
    payload.write_var_i32(12);
    let payload = payload.into_inner();

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_SHOW_DIALOG, &payload).unwrap(),
        PlayClientbound::ShowDialog(ShowDialog {
            dialog: DialogHolder::Reference { registry_id: 11 },
        })
    );
}

#[test]
fn decodes_show_dialog_direct_holder_as_raw_payload() {
    let mut payload = Encoder::new();
    payload.write_var_i32(0);
    payload.write_bytes(&[0x02, 0x03, 0x04]);
    let payload = payload.into_inner();

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_SHOW_DIALOG, &payload).unwrap(),
        PlayClientbound::ShowDialog(ShowDialog {
            dialog: DialogHolder::Direct {
                raw_dialog_payload: vec![0x02, 0x03, 0x04],
            },
        })
    );
}
