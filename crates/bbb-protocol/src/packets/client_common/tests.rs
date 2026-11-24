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

#[test]
fn encode_clear_dialog_is_the_empty_unit_payload() {
    let (id, payload) = encode_play_clientbound_clear_dialog();
    assert_eq!(id, ids::play::CLIENTBOUND_CLEAR_DIALOG);
    assert!(payload.is_empty());
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::ClearDialog
    );
}

#[test]
fn encode_show_dialog_reference_is_the_decoder_inverse() {
    let packet = ShowDialog {
        dialog: DialogHolder::Reference { registry_id: 11 },
    };
    let (id, payload) = encode_play_clientbound_show_dialog(&packet).unwrap();
    assert_eq!(id, ids::play::CLIENTBOUND_SHOW_DIALOG);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::ShowDialog(packet)
    );
}

#[test]
fn encode_show_dialog_direct_is_the_decoder_inverse() {
    let packet = ShowDialog {
        dialog: DialogHolder::Direct {
            dialog: expected_dialog(),
        },
    };
    let (id, payload) = encode_play_clientbound_show_dialog(&packet).unwrap();
    assert_eq!(id, ids::play::CLIENTBOUND_SHOW_DIALOG);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::ShowDialog(packet)
    );
}

#[test]
fn encode_show_dialog_refuses_a_context_free_holder() {
    let packet = ShowDialog {
        dialog: DialogHolder::ContextFree {
            dialog: expected_dialog(),
        },
    };
    let err = encode_play_clientbound_show_dialog(&packet).unwrap_err();
    assert!(err
        .to_string()
        .contains("play show-dialog encode refuses a context-free holder"));
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
        name: "type".into(),
        value: NbtSummaryValue::String("minecraft:notice".into()),
    }])
}

fn write_nbt_string(payload: &mut Encoder, value: &str) {
    payload.write_u16(value.len() as u16);
    payload.write_bytes(value.as_bytes());
}

#[test]
fn custom_click_action_decoder_enforces_vanilla_depth_16_and_32k_quota() {
    // Depth 17 nested compound must be rejected even though wire length is tiny.
    let mut deep_tag = vec![10_u8]; // root TAG_Compound (depth 0)
    for _ in 0..16 {
        deep_tag.push(10); // nested TAG_Compound up to depth 16 (17th compound)
        deep_tag.extend_from_slice(&1u16.to_be_bytes());
        deep_tag.push(b'a');
    }
    deep_tag.extend(std::iter::repeat_n(0_u8, 17));
    let mut payload = Encoder::new();
    payload.write_string("minecraft:test");
    payload.write_var_i32(deep_tag.len() as i32);
    payload.write_bytes(&deep_tag);
    let err =
        decode_custom_click_action_payload(&mut Decoder::new(&payload.into_inner())).unwrap_err();
    assert!(err.to_string().contains("exceeded max depth"), "got {err}");

    // >32,768 accounted bytes inside a <=65,536 wire prefix must be rejected.
    let mut heavy_tag = Vec::new();
    heavy_tag.push(7); // TAG_Byte_Array
    heavy_tag.extend_from_slice(&33_000_i32.to_be_bytes());
    heavy_tag.extend(std::iter::repeat_n(0_u8, 33_000));
    let mut payload = Encoder::new();
    payload.write_string("minecraft:test");
    payload.write_var_i32(heavy_tag.len() as i32);
    payload.write_bytes(&heavy_tag);
    let err =
        decode_custom_click_action_payload(&mut Decoder::new(&payload.into_inner())).unwrap_err();
    assert!(err.to_string().contains("exceeded byte quota"), "got {err}");

    // 10,000-byte array fits within 32,768 accounted bytes and is accepted.
    let mut ok_tag = Vec::new();
    ok_tag.push(7);
    ok_tag.extend_from_slice(&10_000_i32.to_be_bytes());
    ok_tag.extend(std::iter::repeat_n(0_u8, 10_000));
    let mut payload = Encoder::new();
    payload.write_string("minecraft:test");
    payload.write_var_i32(ok_tag.len() as i32);
    payload.write_bytes(&ok_tag);
    let decoded = decode_custom_click_action_payload(&mut Decoder::new(&payload.into_inner()))
        .expect("10 KiB byte array fits in 32 KiB quota");
    assert_eq!(decoded.id, "minecraft:test");
}

#[test]
fn serverbound_custom_payload_enforces_32767_byte_cap_while_clientbound_allows_1mib() {
    let mut max_ok = Encoder::new();
    max_ok.write_string("example:chan");
    max_ok.write_bytes(&vec![0xab; MAX_SERVERBOUND_CUSTOM_PAYLOAD]);
    let max_ok = max_ok.into_inner();
    assert!(crate::packets::decode_configuration_serverbound(
        ids::configuration::SERVERBOUND_CUSTOM_PAYLOAD,
        &max_ok,
    )
    .is_ok());
    assert!(crate::packets::decode_play_serverbound(
        ids::play::SERVERBOUND_CUSTOM_PAYLOAD,
        &max_ok,
    )
    .is_ok());

    let mut oversized = Encoder::new();
    oversized.write_string("example:chan");
    oversized.write_bytes(&vec![0xab; MAX_SERVERBOUND_CUSTOM_PAYLOAD + 1]);
    let oversized = oversized.into_inner();
    assert!(matches!(
        crate::packets::decode_configuration_serverbound(
            ids::configuration::SERVERBOUND_CUSTOM_PAYLOAD,
            &oversized,
        ),
        Err(ProtocolError::PacketTooLarge(32_768, 32_767))
    ));
    assert!(matches!(
        crate::packets::decode_play_serverbound(ids::play::SERVERBOUND_CUSTOM_PAYLOAD, &oversized,),
        Err(ProtocolError::PacketTooLarge(32_768, 32_767))
    ));
    assert!(decode_play_clientbound(ids::play::CLIENTBOUND_CUSTOM_PAYLOAD, &oversized).is_ok());
}
