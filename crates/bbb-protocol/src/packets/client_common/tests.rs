use super::*;
use crate::{
    codec::{Decoder, Encoder, ProtocolError},
    ids,
    nbt_summary::MAX_NBT_COMPOUND_ENTRIES,
    packets::{decode_play_clientbound, PlayClientbound},
    NbtSummaryEntry,
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
fn decodes_show_dialog_direct_holder_as_bounded_nbt() {
    let mut payload = Encoder::new();
    payload.write_var_i32(0);
    payload.write_bytes(&dialog_nbt());
    let payload = payload.into_inner();

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_SHOW_DIALOG, &payload).unwrap(),
        PlayClientbound::ShowDialog(ShowDialog {
            dialog: DialogHolder::Direct {
                dialog: expected_dialog(),
            },
        })
    );
}

#[test]
fn rejects_show_dialog_direct_holder_with_non_compound_root() {
    let mut payload = Encoder::new();
    payload.write_var_i32(0);
    payload.write_u8(3);
    payload.write_i32(4);

    let error = decode_play_clientbound(ids::play::CLIENTBOUND_SHOW_DIALOG, &payload.into_inner())
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("direct dialog payload root must be a compound tag"));
}

#[test]
fn rejects_show_dialog_direct_holder_with_trailing_bytes() {
    let mut payload = Encoder::new();
    payload.write_var_i32(0);
    payload.write_bytes(&dialog_nbt());
    payload.write_u8(0xff);

    let error = decode_play_clientbound(ids::play::CLIENTBOUND_SHOW_DIALOG, &payload.into_inner())
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("trailing bytes after direct dialog payload"));
}

#[test]
fn rejects_oversized_show_dialog_direct_holder_before_nbt_allocation() {
    let mut payload = Encoder::new();
    payload.write_var_i32(0);
    payload.write_bytes(&vec![0; MAX_DIALOG_PAYLOAD + 1]);

    let error = decode_play_clientbound(ids::play::CLIENTBOUND_SHOW_DIALOG, &payload.into_inner())
        .unwrap_err();
    assert!(matches!(
        error,
        ProtocolError::PacketTooLarge(actual, max)
            if actual == MAX_DIALOG_PAYLOAD + 1 && max == MAX_DIALOG_PAYLOAD
    ));
}

#[test]
fn rejects_show_dialog_direct_holder_with_excessive_compound_entries() {
    let mut payload = Encoder::new();
    payload.write_var_i32(0);
    payload.write_u8(10);
    for _ in 0..=MAX_NBT_COMPOUND_ENTRIES {
        payload.write_u8(1);
        payload.write_u16(0);
        payload.write_i8(0);
    }
    payload.write_u8(0);

    let error = decode_play_clientbound(ids::play::CLIENTBOUND_SHOW_DIALOG, &payload.into_inner())
        .unwrap_err();
    assert!(matches!(
        error,
        ProtocolError::PacketTooLarge(actual, max)
            if actual == MAX_NBT_COMPOUND_ENTRIES + 1 && max == MAX_NBT_COMPOUND_ENTRIES
    ));
}

fn dialog_nbt() -> Vec<u8> {
    let mut payload = Encoder::new();
    payload.write_u8(10);
    payload.write_u8(8);
    write_nbt_string(&mut payload, "type");
    write_nbt_string(&mut payload, "minecraft:notice");
    payload.write_u8(0);
    payload.into_inner()
}

fn expected_dialog() -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![NbtSummaryEntry {
        name: "type".to_string(),
        value: NbtSummaryValue::String("minecraft:notice".to_string()),
    }])
}

fn write_nbt_string(payload: &mut Encoder, value: &str) {
    payload.write_u16(value.len() as u16);
    payload.write_bytes(value.as_bytes());
}
