use crate::{
    decode_client_known_pack_registry_json, decode_direct_registry_json,
    decode_network_registry_element, registry_codec::recursive_values::RegistryCodecDecodeContext,
};
use bbb_protocol::encode_nbt_summary;

use super::*;

fn decode(json: &[u8]) -> LootConditionCodecValue {
    let data = decode_direct_registry_json(json).unwrap();
    decode_data(&data)
}

fn decode_data(data: &crate::registry_codec::RegistryElementData) -> LootConditionCodecValue {
    let mut context = RegistryCodecDecodeContext::new(
        "minecraft:enchantment",
        "LootItemCondition.DIRECT_CODEC",
        data.source,
    );
    decode_loot_condition_source(data.root(), "$", &mut context).unwrap()
}

fn compound(fields: Vec<bbb_protocol::NbtSummaryEntry>) -> bbb_protocol::NbtSummaryValue {
    bbb_protocol::NbtSummaryValue::Compound(fields)
}

fn string(value: &str) -> bbb_protocol::NbtSummaryValue {
    bbb_protocol::NbtSummaryValue::String(value.to_string().into())
}

fn network(value: &bbb_protocol::NbtSummaryValue) -> crate::registry_codec::RegistryElementData {
    decode_network_registry_element(&encode_nbt_summary(value, 1 << 20).unwrap()).unwrap()
}

#[test]
fn recursive_and_reference_families_have_exact_three_source_writers() {
    let cases = [
        (
            br#"{"condition":"inverted","term":{"condition":"weather_check","raining":false}}"#
                .as_slice(),
            compound(vec![
                entry(
                    "term",
                    compound(vec![
                        entry("raining", bbb_protocol::NbtSummaryValue::Byte(0)),
                        entry("condition", string("minecraft:weather_check")),
                    ]),
                ),
                entry("condition", string("minecraft:inverted")),
            ]),
            None,
        ),
        (
            br#"{"condition":"any_of","terms":[]}"#.as_slice(),
            compound(vec![
                entry("terms", bbb_protocol::NbtSummaryValue::List(vec![])),
                entry("condition", string("minecraft:any_of")),
            ]),
            None,
        ),
        (
            br#"{"condition":"all_of","terms":[]}"#.as_slice(),
            compound(vec![
                entry("terms", bbb_protocol::NbtSummaryValue::List(vec![])),
                entry("condition", string("minecraft:all_of")),
            ]),
            None,
        ),
        (
            br#"{"condition":"reference","name":"example:other"}"#.as_slice(),
            compound(vec![
                entry("name", string("example:other")),
                entry("condition", string("minecraft:reference")),
            ]),
            Some("example:other"),
        ),
    ];
    for (json, expected, expected_reference) in cases {
        let direct = decode_direct_registry_json(json).unwrap();
        let known = decode_client_known_pack_registry_json(json).unwrap();
        let wire = network(&expected);
        for data in [&direct, &known, &wire] {
            let decoded = decode_data(data);
            assert_eq!(decoded.value.canonical.network_nbt_unchecked(), &expected);
            assert_eq!(
                decoded.has_predicate_reference,
                expected_reference.is_some(),
            );
        }
    }
}

#[test]
fn inline_all_of_reencodes_through_the_primary_typed_branch() {
    let inline = decode(br#"[{"condition":"weather_check","raining":false}]"#);
    let typed = decode(
        br#"{"condition":"all_of","terms":[{"condition":"weather_check","raining":false}]}"#,
    );
    assert_eq!(inline, typed);
    let bbb_protocol::NbtSummaryValue::Compound(fields) =
        inline.value.canonical.network_nbt_unchecked()
    else {
        panic!("all_of must write a typed compound")
    };
    assert_eq!(fields.last().unwrap().name, "condition");
    assert_eq!(fields.first().unwrap().name, "terms");

    let inline_reference = decode(br#"[{"condition":"reference","name":"example:other"}]"#);
    let typed_reference = decode(
        br#"{"condition":"all_of","terms":[{"condition":"reference","name":"example:other"}]}"#,
    );
    assert_eq!(inline_reference, typed_reference);
}
