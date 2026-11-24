use super::*;
use crate::{
    decode_client_known_pack_registry_json, decode_direct_registry_json,
    decode_network_registry_element,
    registry_codec::{recursive_values::RegistryCodecDecodeContext, RegistryElementData},
};
use bbb_protocol::{encode_nbt_summary, NbtSummaryEntry, NbtSummaryValue};

fn decode(json: &str) -> LootContextCodecValue {
    decode_data(&decode_direct_registry_json(json.as_bytes()).unwrap())
}
fn decode_data(data: &RegistryElementData) -> LootContextCodecValue {
    let mut context = RegistryCodecDecodeContext::new(
        "minecraft:loot_table",
        "LootItemCondition.CODEC",
        data.source,
    );
    decode_int_range_source(data.root(), "$", &mut context).unwrap()
}

fn network(value: NbtSummaryValue) -> RegistryElementData {
    decode_network_registry_element(&encode_nbt_summary(&value, 1 << 20).unwrap()).unwrap()
}
fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(entries)
}

#[test]
fn scalar_and_equal_semantic_constants_pack_to_int() {
    assert_eq!(
        decode("4").canonical.network_nbt_unchecked(),
        &NbtSummaryValue::Int(4)
    );
    assert_eq!(
        decode(r#"{"min":{"type":"constant","value":4},"max":4}"#)
            .canonical
            .network_nbt_unchecked(),
        &NbtSummaryValue::Int(4)
    );
    assert_eq!(
        decode(r#"{"min":-0.0,"max":-0.0}"#)
            .canonical
            .network_nbt_unchecked(),
        &NbtSummaryValue::Int(0)
    );
    assert_eq!(
        decode(r#"{"min":0.0,"max":0.0}"#)
            .canonical
            .network_nbt_unchecked(),
        &NbtSummaryValue::Int(0)
    );
}

#[test]
fn three_sources_share_scalar_record_and_empty_writer_shapes() {
    let cases = [
        (
            br#"4"#.as_slice(),
            NbtSummaryValue::Int(4),
            NbtSummaryValue::Int(4),
        ),
        (
            br#"{"min":2,"max":4}"#.as_slice(),
            compound(vec![
                entry("min", NbtSummaryValue::Float(2.0_f32.to_bits())),
                entry("max", NbtSummaryValue::Float(4.0_f32.to_bits())),
            ]),
            compound(vec![
                entry("min", NbtSummaryValue::Float(2.0_f32.to_bits())),
                entry("max", NbtSummaryValue::Float(4.0_f32.to_bits())),
            ]),
        ),
        (br#"{}"#.as_slice(), compound(vec![]), compound(vec![])),
    ];
    for (json, input, expected) in cases {
        for data in [
            decode_direct_registry_json(json).unwrap(),
            decode_client_known_pack_registry_json(json).unwrap(),
            network(input),
        ] {
            assert_eq!(
                decode_data(&data).canonical.network_nbt_unchecked(),
                &expected
            );
        }
    }
}
