use crate::{
    decode_client_known_pack_registry_json, decode_direct_registry_json,
    decode_network_registry_element,
    registry_codec::{
        recursive_values::RegistryCodecDecodeContext, RegistryElementData, RegistryElementSource,
    },
};
use bbb_protocol::{encode_nbt_summary, nbt_summary::entry, NbtSummaryEntry};

use super::*;

const OWNER_REGISTRY: &str = "minecraft:enchantment";
const OWNER_CODEC: &str = "Enchantment.DIRECT_CODEC";

fn context(source: RegistryElementSource) -> RegistryCodecDecodeContext<'static> {
    RegistryCodecDecodeContext::new(OWNER_REGISTRY, OWNER_CODEC, source)
}

fn decode_level(data: &RegistryElementData) -> CanonicalRegistryCodecValue {
    decode_level_based_value_source(data.root(), "$", &mut context(data.source)).unwrap()
}

fn decode_effect(data: &RegistryElementData) -> CanonicalRegistryCodecValue {
    decode_enchantment_value_effect_source(data.root(), "$", &mut context(data.source)).unwrap()
}

fn network_data(value: &NbtSummaryValue) -> RegistryElementData {
    decode_network_registry_element(&encode_nbt_summary(value, 1 << 20).unwrap()).unwrap()
}

fn float(value: f32) -> NbtSummaryValue {
    NbtSummaryValue::Float(bbb_protocol::nbt_summary::canonical_float_tag_bits(value))
}

fn compound(fields: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(fields)
}

#[test]
fn all_level_based_value_forms_write_codec_field_order_and_normalized_type_last() {
    let cases = [
        (br#"2.5"#.as_slice(), float(2.5)),
        (
            br#"{"type":"clamped","value":1,"min":0,"max":2,"ignored":true}"#.as_slice(),
            compound(vec![
                entry("value", float(1.0)),
                entry("min", float(0.0)),
                entry("max", float(2.0)),
                entry(
                    "type",
                    NbtSummaryValue::String("minecraft:clamped".to_string().into()),
                ),
            ]),
        ),
        (
            br#"{"type":"fraction","numerator":1,"denominator":2}"#.as_slice(),
            compound(vec![
                entry("numerator", float(1.0)),
                entry("denominator", float(2.0)),
                entry(
                    "type",
                    NbtSummaryValue::String("minecraft:fraction".to_string().into()),
                ),
            ]),
        ),
        (
            br#"{"type":"levels_squared","added":1}"#.as_slice(),
            compound(vec![
                entry("added", float(1.0)),
                entry(
                    "type",
                    NbtSummaryValue::String("minecraft:levels_squared".to_string().into()),
                ),
            ]),
        ),
        (
            br#"{"type":"linear","base":1,"per_level_above_first":2}"#.as_slice(),
            compound(vec![
                entry("base", float(1.0)),
                entry("per_level_above_first", float(2.0)),
                entry(
                    "type",
                    NbtSummaryValue::String("minecraft:linear".to_string().into()),
                ),
            ]),
        ),
        (
            br#"{"type":"exponent","base":2,"power":3}"#.as_slice(),
            compound(vec![
                entry("base", float(2.0)),
                entry("power", float(3.0)),
                entry(
                    "type",
                    NbtSummaryValue::String("minecraft:exponent".to_string().into()),
                ),
            ]),
        ),
        (
            br#"{"type":"lookup","values":[1,2],"fallback":3}"#.as_slice(),
            compound(vec![
                entry(
                    "values",
                    NbtSummaryValue::List(vec![float(1.0), float(2.0)]),
                ),
                entry("fallback", float(3.0)),
                entry(
                    "type",
                    NbtSummaryValue::String("minecraft:lookup".to_string().into()),
                ),
            ]),
        ),
    ];
    for (json, expected) in cases {
        for data in [
            decode_direct_registry_json(json).unwrap(),
            decode_client_known_pack_registry_json(json).unwrap(),
            network_data(&expected),
        ] {
            let decoded = decode_level(&data);
            assert_eq!(decoded.network_nbt_unchecked(), &expected, "{json:?}");
            assert!(decoded.references().next().is_none());
        }
    }
}

#[test]
fn all_value_effect_forms_write_payload_then_type_and_allow_empty_all_of() {
    let cases = [
        (
            br#"{"type":"add","value":1}"#.as_slice(),
            compound(vec![
                entry("value", float(1.0)),
                entry(
                    "type",
                    NbtSummaryValue::String("minecraft:add".to_string().into()),
                ),
            ]),
        ),
        (
            br#"{"type":"all_of","effects":[]}"#.as_slice(),
            compound(vec![
                entry("effects", NbtSummaryValue::List(Vec::new())),
                entry(
                    "type",
                    NbtSummaryValue::String("minecraft:all_of".to_string().into()),
                ),
            ]),
        ),
        (
            br#"{"type":"multiply","factor":1}"#.as_slice(),
            compound(vec![
                entry("factor", float(1.0)),
                entry(
                    "type",
                    NbtSummaryValue::String("minecraft:multiply".to_string().into()),
                ),
            ]),
        ),
        (
            br#"{"type":"remove_binomial","chance":0.5}"#.as_slice(),
            compound(vec![
                entry("chance", float(0.5)),
                entry(
                    "type",
                    NbtSummaryValue::String("minecraft:remove_binomial".to_string().into()),
                ),
            ]),
        ),
        (
            br#"{"type":"exponential","base":2,"exponent":3}"#.as_slice(),
            compound(vec![
                entry("base", float(2.0)),
                entry("exponent", float(3.0)),
                entry(
                    "type",
                    NbtSummaryValue::String("minecraft:exponential".to_string().into()),
                ),
            ]),
        ),
        (
            br#"{"type":"set","value":1}"#.as_slice(),
            compound(vec![
                entry("value", float(1.0)),
                entry(
                    "type",
                    NbtSummaryValue::String("minecraft:set".to_string().into()),
                ),
            ]),
        ),
    ];
    for (json, expected) in cases {
        for data in [
            decode_direct_registry_json(json).unwrap(),
            decode_client_known_pack_registry_json(json).unwrap(),
            network_data(&expected),
        ] {
            let decoded = decode_effect(&data);
            assert_eq!(decoded.network_nbt_unchecked(), &expected, "{json:?}");
            assert!(decoded.references().next().is_none());
        }
    }
}

#[test]
fn server_known_pack_and_network_sources_round_trip_to_one_canonical_value() {
    let json = br#"{
        "type":"exponential",
        "base":{"type":"lookup","values":[1,-0.0,1e400],"fallback":2},
        "exponent":{"type":"fraction","numerator":3,"denominator":4},
        "ignored":"dropped"
    }"#;
    let server = decode_effect(&decode_direct_registry_json(json).unwrap());
    let known_pack = decode_effect(&decode_client_known_pack_registry_json(json).unwrap());
    assert_eq!(server, known_pack);

    let network = network_data(server.network_nbt_unchecked());
    let network = decode_effect(&network);
    assert_eq!(server, network);
}
