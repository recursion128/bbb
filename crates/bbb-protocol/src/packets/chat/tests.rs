use super::*;
use crate::{
    codec::Encoder,
    ids,
    packets::{decode_play_clientbound, PlayClientbound},
};
use uuid::Uuid;

#[test]
fn decodes_delete_chat_with_full_signature() {
    let mut payload = Encoder::new();
    payload.write_var_i32(0);
    payload.write_bytes(&[7; 256]);

    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_DELETE_CHAT, &payload.into_inner()).unwrap();

    assert_eq!(
        packet,
        PlayClientbound::DeleteChat(DeleteChat {
            message_signature: PackedMessageSignature {
                cache_id: None,
                full_signature: Some(MessageSignature {
                    bytes: vec![7; 256],
                }),
            },
        })
    );
}

#[test]
fn decodes_disguised_chat_with_bound_chat_type() {
    let mut payload = Encoder::new();
    payload.write_bytes(&nbt_string_root("Server says hello"));
    write_chat_type_bound(&mut payload, 3, "Server", None);

    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_DISGUISED_CHAT, &payload.into_inner())
            .unwrap();

    assert_eq!(
        packet,
        PlayClientbound::DisguisedChat(DisguisedChat {
            message: "Server says hello".to_string(),
            chat_type: ChatTypeBound {
                chat_type: ChatTypeHolder::Registry { id: 2 },
                name: "Server".to_string(),
                target_name: None,
            },
        })
    );
}

#[test]
fn decodes_disguised_chat_with_direct_chat_type() {
    let mut payload = Encoder::new();
    payload.write_bytes(&nbt_string_root("Direct hello"));
    payload.write_var_i32(0);
    write_chat_type_decoration(&mut payload, "chat.type.text", &[0, 2]);
    write_chat_type_decoration(&mut payload, "chat.type.text.narrate", &[0, 2]);
    payload.write_bytes(&nbt_string_root("Narrator"));
    payload.write_bool(false);

    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_DISGUISED_CHAT, &payload.into_inner())
            .unwrap();

    assert_eq!(
        packet,
        PlayClientbound::DisguisedChat(DisguisedChat {
            message: "Direct hello".to_string(),
            chat_type: ChatTypeBound {
                chat_type: ChatTypeHolder::Direct {
                    chat: ChatTypeDecorationSummary {
                        translation_key: "chat.type.text".to_string(),
                        parameters: vec![0, 2],
                    },
                    narration: ChatTypeDecorationSummary {
                        translation_key: "chat.type.text.narrate".to_string(),
                        parameters: vec![0, 2],
                    },
                },
                name: "Narrator".to_string(),
                target_name: None,
            },
        })
    );
}

#[test]
fn decodes_player_chat_wire_order() {
    let sender = Uuid::from_u128(0x00112233445566778899aabbccddeeff);
    let mut payload = Encoder::new();
    payload.write_var_i32(4);
    payload.write_uuid(sender);
    payload.write_var_i32(9);
    payload.write_bool(true);
    payload.write_bytes(&[3; 256]);
    payload.write_string("hello");
    payload.write_i64(1_700_000_000_123);
    payload.write_i64(42);
    payload.write_var_i32(1);
    payload.write_var_i32(2);
    payload.write_bool(true);
    payload.write_bytes(&nbt_string_root("unsigned hello"));
    payload.write_var_i32(2);
    payload.write_var_i32(1);
    payload.write_i64(0b101);
    write_chat_type_bound(&mut payload, 1, "Alice", Some("Bob"));

    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_PLAYER_CHAT, &payload.into_inner()).unwrap();

    assert_eq!(
        packet,
        PlayClientbound::PlayerChat(PlayerChat {
            global_index: 4,
            sender,
            index: 9,
            signature: Some(MessageSignature {
                bytes: vec![3; 256],
            }),
            body: SignedMessageBody {
                content: "hello".to_string(),
                timestamp_millis: 1_700_000_000_123,
                salt: 42,
                last_seen: vec![PackedMessageSignature {
                    cache_id: Some(1),
                    full_signature: None,
                }],
            },
            unsigned_content: Some("unsigned hello".to_string()),
            filter_mask: FilterMask {
                kind: FilterMaskKind::PartiallyFiltered,
                mask_words: vec![0b101],
            },
            chat_type: ChatTypeBound {
                chat_type: ChatTypeHolder::Registry { id: 0 },
                name: "Alice".to_string(),
                target_name: Some("Bob".to_string()),
            },
        })
    );
}

#[test]
fn rejects_unknown_filter_mask_type() {
    let sender = Uuid::from_u128(1);
    let mut payload = Encoder::new();
    payload.write_var_i32(0);
    payload.write_uuid(sender);
    payload.write_var_i32(0);
    payload.write_bool(false);
    payload.write_string("hello");
    payload.write_i64(1);
    payload.write_i64(2);
    payload.write_var_i32(0);
    payload.write_bool(false);
    payload.write_var_i32(3);

    let err = decode_play_clientbound(ids::play::CLIENTBOUND_PLAYER_CHAT, &payload.into_inner())
        .unwrap_err();
    assert!(err
        .to_string()
        .contains("invalid filter mask type ordinal 3"));
}

#[test]
fn rejects_negative_packed_message_signature_id() {
    let mut payload = Encoder::new();
    payload.write_var_i32(-1);

    let err = decode_play_clientbound(ids::play::CLIENTBOUND_DELETE_CHAT, &payload.into_inner())
        .unwrap_err();
    assert!(err
        .to_string()
        .contains("invalid packed message signature id -2"));
}

fn write_chat_type_bound(
    payload: &mut Encoder,
    holder_id: i32,
    name: &str,
    target_name: Option<&str>,
) {
    payload.write_var_i32(holder_id);
    payload.write_bytes(&nbt_string_root(name));
    match target_name {
        Some(target_name) => {
            payload.write_bool(true);
            payload.write_bytes(&nbt_string_root(target_name));
        }
        None => payload.write_bool(false),
    }
}

fn write_chat_type_decoration(payload: &mut Encoder, translation_key: &str, parameters: &[i32]) {
    payload.write_string(translation_key);
    payload.write_var_i32(parameters.len() as i32);
    for parameter in parameters {
        payload.write_var_i32(*parameter);
    }
    payload.write_bytes(&empty_nbt_compound());
}

fn nbt_string_root(text: &str) -> Vec<u8> {
    let mut out = vec![8];
    out.extend_from_slice(&(text.len() as u16).to_be_bytes());
    out.extend_from_slice(text.as_bytes());
    out
}

fn empty_nbt_compound() -> Vec<u8> {
    vec![10, 0]
}
