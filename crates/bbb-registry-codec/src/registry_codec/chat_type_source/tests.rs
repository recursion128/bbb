use super::*;
use crate::registry_codec::{
    decode_client_known_pack_registry_json, decode_direct_registry_json, RegistryElementSource,
};
use bbb_protocol::NbtSummaryEntry;

fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::String(value.to_string().into())
}

fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(entries)
}

fn network_data(value: NbtSummaryValue) -> RegistryElementData {
    RegistryElementData::from_network_nbt(value)
}

fn names(value: &NbtSummaryValue) -> Vec<&bbb_protocol::JavaString> {
    let NbtSummaryValue::Compound(entries) = value else {
        panic!("expected a compound")
    };
    entries.iter().map(|entry| &entry.name).collect()
}

fn required_decoration(key: &str) -> NbtSummaryValue {
    compound(vec![
        entry("translation_key", string(key)),
        entry(
            "parameters",
            NbtSummaryValue::List(vec![string("sender"), string("content")]),
        ),
    ])
}

fn basic_network_value() -> NbtSummaryValue {
    compound(vec![
        entry("chat", required_decoration("chat.type.text")),
        entry("narration", required_decoration("chat.type.text.narrate")),
    ])
}

#[test]
fn all_three_sources_round_trip_to_direct_codec_order() {
    let raw = decode_chat_type_element(&decode_direct_registry_json(
        br#"{"chat":{"translation_key":"\ud800\ud801\ufffd","parameters":[]},"narration":{"translation_key":"plain","parameters":[]}}"#,
    ).unwrap()).unwrap();
    let NbtSummaryValue::Compound(root) = raw.network_nbt_unchecked() else {
        panic!("expected compound");
    };
    let NbtSummaryValue::Compound(chat) = &root[0].value else {
        panic!("expected chat decoration");
    };
    assert_eq!(
        chat[0].value,
        bbb_protocol::JavaString::from(vec![0xd800, 0xd801, 0xfffd]).to_nbt()
    );
    assert_eq!(
        decode_chat_type_element(&network_data(raw.network_nbt_unchecked().clone()))
            .unwrap()
            .network_nbt_unchecked(),
        raw.network_nbt_unchecked()
    );

    let json = br#"{
        "narration":{"parameters":["sender","content"],"translation_key":"chat.type.text.narrate"},
        "ignored":{"deep":[1,2,3]},
        "chat":{"unknown":true,"parameters":["sender","content"],"translation_key":"chat.type.text"}
    }"#;
    let expected = basic_network_value();
    for data in [
        decode_direct_registry_json(json).unwrap(),
        decode_client_known_pack_registry_json(json).unwrap(),
        network_data(expected.clone()),
    ] {
        let decoded = decode_chat_type_element(&data).unwrap();
        assert_eq!(decoded.network_nbt_unchecked(), &expected);
        assert_eq!(
            names(decoded.network_nbt_unchecked()),
            ["chat", "narration"]
        );
    }

    let round_trip = decode_chat_type_element(&network_data(expected.clone())).unwrap();
    assert_eq!(round_trip.network_nbt_unchecked(), &expected);
    assert_eq!(
        synchronized_registry_element_codec(
            CHAT_TYPE_REGISTRY,
            RegistryElementSource::ServerDatapackJson
        )
        .unwrap(),
        "ChatType.DIRECT_CODEC"
    );
    assert_eq!(
        synchronized_registry_element_codec(CHAT_TYPE_REGISTRY, RegistryElementSource::NetworkNbt)
            .unwrap(),
        "ChatType.DIRECT_CODEC"
    );
}

#[test]
fn parameter_enum_is_exact_and_preserves_list_order() {
    let json = br#"{
        "chat":{"translation_key":"chat","parameters":["target","sender","content"]},
        "narration":{"translation_key":"narration","parameters":[]}
    }"#;
    let decoded = decode_chat_type_element(&decode_direct_registry_json(json).unwrap()).unwrap();
    let NbtSummaryValue::Compound(root) = decoded.network_nbt_unchecked() else {
        panic!("expected root compound")
    };
    let NbtSummaryValue::Compound(chat) = &root[0].value else {
        panic!("expected chat decoration")
    };
    assert_eq!(
        chat[1].value,
        NbtSummaryValue::List(vec![string("target"), string("sender"), string("content")])
    );

    for parameter in ["SENDER", "other"] {
        let json = format!(
            r#"{{"chat":{{"translation_key":"chat","parameters":["{parameter}"]}},"narration":{{"translation_key":"narration","parameters":[]}}}}"#
        );
        assert!(
            decode_chat_type_element(&decode_direct_registry_json(json.as_bytes()).unwrap())
                .is_err()
        );
    }
}
