use crate::{
    decode_client_known_pack_registry_json, decode_direct_registry_json,
    decode_network_registry_element, RegistryElementData, RegistryElementSource,
};
use bbb_protocol::{encode_nbt_summary, NbtSummaryEntry, NbtSummaryValue};

use super::*;

const TEST_REGISTRY: &str = "minecraft:enchantment";
const TEST_CODEC: &str = "Enchantment.DIRECT_CODEC";

fn server(json: &str) -> RegistryElementData {
    decode_direct_registry_json(json.as_bytes()).unwrap()
}

fn known(json: &str) -> RegistryElementData {
    decode_client_known_pack_registry_json(json.as_bytes()).unwrap()
}

fn network(value: &NbtSummaryValue) -> RegistryElementData {
    let wire = encode_nbt_summary(value, 8 * 1024 * 1024).unwrap();
    decode_network_registry_element(&wire).unwrap()
}

fn context(source: RegistryElementSource) -> RegistryCodecDecodeContext<'static> {
    RegistryCodecDecodeContext::new(TEST_REGISTRY, TEST_CODEC, source)
}

fn decode_int(
    data: &RegistryElementData,
    minimum: i32,
    maximum: i32,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    decode_int_provider_source(
        data.root(),
        "$",
        &mut context(data.source),
        minimum,
        maximum,
    )
}

fn decode_float(
    data: &RegistryElementData,
    minimum: f32,
    maximum: f32,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    decode_float_provider_source(
        data.root(),
        "$",
        &mut context(data.source),
        minimum,
        maximum,
    )
}

fn assert_int_all_sources(
    json: &str,
    nbt: &NbtSummaryValue,
    expected: &NbtSummaryValue,
    min: i32,
    max: i32,
) {
    for data in [server(json), known(json), network(nbt)] {
        let provider = match data.root() {
            RegistryElementValueRef::Json(value) => IntProvider::decode(value),
            input => decode_int_provider_value(input, "$", &mut context(data.source)),
        }
        .unwrap();
        assert_eq!(
            (provider.min_inclusive(), provider.max_inclusive()),
            (min, max)
        );
        assert_eq!(&provider.network_nbt(), expected);
        assert_eq!(&provider.into_nbt(), expected);
        let decoded = decode_int(&data, i32::MIN, i32::MAX).unwrap();
        assert_eq!(decoded.network_nbt_unchecked(), expected);
    }
}

fn assert_float_all_sources(
    json: &str,
    nbt: &NbtSummaryValue,
    expected: &NbtSummaryValue,
    min: f32,
    max: f32,
) {
    for data in [server(json), known(json), network(nbt)] {
        let provider = FloatProvider::decode_source(data.root()).unwrap();
        assert_eq!(&provider.into_nbt(), expected);
        assert_eq!(provider.bounds(), (min, max));
        let decoded = decode_float(&data, f32::NEG_INFINITY, f32::INFINITY).unwrap();
        assert_eq!(decoded.network_nbt_unchecked(), expected);
    }
}

fn compound(fields: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(fields)
}

fn list(values: Vec<NbtSummaryValue>) -> NbtSummaryValue {
    NbtSummaryValue::List(values)
}

fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::String(value.to_string().into())
}

fn int(value: i32) -> NbtSummaryValue {
    NbtSummaryValue::Int(value)
}

fn float_tag(value: f32) -> NbtSummaryValue {
    NbtSummaryValue::Float(canonical_float_tag_bits(value))
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

#[test]
fn all_seven_int_dispatch_families_and_inline_constant_are_exact_in_three_sources() {
    let cases = [
        ("4", int(4), int(4), 4, 4),
        (
            r#"{"type":"constant","value":4}"#,
            compound(vec![
                entry("type", string("constant")),
                entry("value", int(4)),
            ]),
            int(4),
            4,
            4,
        ),
        (
            r#"{"type":"uniform","min_inclusive":1,"max_inclusive":3}"#,
            compound(vec![
                entry("type", string("uniform")),
                entry("min_inclusive", int(1)),
                entry("max_inclusive", int(3)),
            ]),
            compound(vec![
                entry("min_inclusive", int(1)),
                entry("max_inclusive", int(3)),
                entry("type", string("minecraft:uniform")),
            ]),
            1,
            3,
        ),
        (
            r#"{"type":"biased_to_bottom","min_inclusive":2,"max_inclusive":5}"#,
            compound(vec![
                entry("type", string("biased_to_bottom")),
                entry("min_inclusive", int(2)),
                entry("max_inclusive", int(5)),
            ]),
            compound(vec![
                entry("min_inclusive", int(2)),
                entry("max_inclusive", int(5)),
                entry("type", string("minecraft:biased_to_bottom")),
            ]),
            2,
            5,
        ),
        (
            r#"{"type":"clamped","source":{"type":"uniform","min_inclusive":1,"max_inclusive":5},"min_inclusive":2,"max_inclusive":4}"#,
            compound(vec![
                entry("type", string("clamped")),
                entry(
                    "source",
                    compound(vec![
                        entry("type", string("uniform")),
                        entry("min_inclusive", int(1)),
                        entry("max_inclusive", int(5)),
                    ]),
                ),
                entry("min_inclusive", int(2)),
                entry("max_inclusive", int(4)),
            ]),
            compound(vec![
                entry(
                    "source",
                    compound(vec![
                        entry("min_inclusive", int(1)),
                        entry("max_inclusive", int(5)),
                        entry("type", string("minecraft:uniform")),
                    ]),
                ),
                entry("min_inclusive", int(2)),
                entry("max_inclusive", int(4)),
                entry("type", string("minecraft:clamped")),
            ]),
            2,
            4,
        ),
        (
            r#"{"type":"weighted_list","distribution":[{"data":7,"weight":1},{"data":{"type":"uniform","min_inclusive":2,"max_inclusive":4},"weight":2}]}"#,
            compound(vec![
                entry("type", string("weighted_list")),
                entry(
                    "distribution",
                    list(vec![
                        compound(vec![entry("data", int(7)), entry("weight", int(1))]),
                        compound(vec![
                            entry(
                                "data",
                                compound(vec![
                                    entry("type", string("uniform")),
                                    entry("min_inclusive", int(2)),
                                    entry("max_inclusive", int(4)),
                                ]),
                            ),
                            entry("weight", int(2)),
                        ]),
                    ]),
                ),
            ]),
            compound(vec![
                entry(
                    "distribution",
                    list(vec![
                        compound(vec![entry("data", int(7)), entry("weight", int(1))]),
                        compound(vec![
                            entry(
                                "data",
                                compound(vec![
                                    entry("min_inclusive", int(2)),
                                    entry("max_inclusive", int(4)),
                                    entry("type", string("minecraft:uniform")),
                                ]),
                            ),
                            entry("weight", int(2)),
                        ]),
                    ]),
                ),
                entry("type", string("minecraft:weighted_list")),
            ]),
            2,
            7,
        ),
        (
            r#"{"type":"clamped_normal","mean":2.5,"deviation":0.5,"min_inclusive":1,"max_inclusive":4}"#,
            compound(vec![
                entry("type", string("clamped_normal")),
                entry("mean", float_tag(2.5)),
                entry("deviation", float_tag(0.5)),
                entry("min_inclusive", int(1)),
                entry("max_inclusive", int(4)),
            ]),
            compound(vec![
                entry("mean", float_tag(2.5)),
                entry("deviation", float_tag(0.5)),
                entry("min_inclusive", int(1)),
                entry("max_inclusive", int(4)),
                entry("type", string("minecraft:clamped_normal")),
            ]),
            1,
            4,
        ),
        (
            r#"{"type":"trapezoid","min":-3,"max":5,"plateau":2}"#,
            compound(vec![
                entry("type", string("trapezoid")),
                entry("min", int(-3)),
                entry("max", int(5)),
                entry("plateau", int(2)),
            ]),
            compound(vec![
                entry("min", int(-3)),
                entry("max", int(5)),
                entry("plateau", int(2)),
                entry("type", string("minecraft:trapezoid")),
            ]),
            -3,
            5,
        ),
    ];

    for (json, nbt, expected, min, max) in cases {
        assert_int_all_sources(json, &nbt, &expected, min, max);
    }
}

#[test]
fn all_four_float_dispatch_families_and_inline_constant_are_exact_in_three_sources() {
    let cases = [
        ("1.25", float_tag(1.25), float_tag(1.25), 1.25, 1.25),
        (
            r#"{"type":"constant","value":1.25}"#,
            compound(vec![
                entry("type", string("constant")),
                entry("value", float_tag(1.25)),
            ]),
            float_tag(1.25),
            1.25,
            1.25,
        ),
        (
            r#"{"type":"uniform","min_inclusive":1.0,"max_exclusive":3.0}"#,
            compound(vec![
                entry("type", string("uniform")),
                entry("min_inclusive", float_tag(1.0)),
                entry("max_exclusive", float_tag(3.0)),
            ]),
            compound(vec![
                entry("min_inclusive", float_tag(1.0)),
                entry("max_exclusive", float_tag(3.0)),
                entry("type", string("minecraft:uniform")),
            ]),
            1.0,
            3.0,
        ),
        (
            r#"{"type":"clamped_normal","mean":2.0,"deviation":0.5,"min":1.0,"max":4.0}"#,
            compound(vec![
                entry("type", string("clamped_normal")),
                entry("mean", float_tag(2.0)),
                entry("deviation", float_tag(0.5)),
                entry("min", float_tag(1.0)),
                entry("max", float_tag(4.0)),
            ]),
            compound(vec![
                entry("mean", float_tag(2.0)),
                entry("deviation", float_tag(0.5)),
                entry("min", float_tag(1.0)),
                entry("max", float_tag(4.0)),
                entry("type", string("minecraft:clamped_normal")),
            ]),
            1.0,
            4.0,
        ),
        (
            r#"{"type":"trapezoid","min":-1.0,"max":5.0,"plateau":2.0}"#,
            compound(vec![
                entry("type", string("trapezoid")),
                entry("min", float_tag(-1.0)),
                entry("max", float_tag(5.0)),
                entry("plateau", float_tag(2.0)),
            ]),
            compound(vec![
                entry("min", float_tag(-1.0)),
                entry("max", float_tag(5.0)),
                entry("plateau", float_tag(2.0)),
                entry("type", string("minecraft:trapezoid")),
            ]),
            -1.0,
            5.0,
        ),
    ];

    for (json, nbt, expected, min, max) in cases {
        assert_float_all_sources(json, &nbt, &expected, min, max);
    }
}
