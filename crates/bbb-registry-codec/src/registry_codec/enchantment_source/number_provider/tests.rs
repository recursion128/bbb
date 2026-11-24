use crate::{
    decode_client_known_pack_registry_json, decode_direct_registry_json,
    decode_network_registry_element,
    loot_context_types::{vanilla_loot_entity_target, VanillaLootContextParam},
    registry_codec::{
        recursive_values::RegistryCodecDecodeContext, RegistryElementData, RegistryElementSource,
    },
};
use bbb_protocol::{
    encode_nbt_summary,
    nbt_summary::{canonical_float_tag_bits, entry},
    NbtSummaryEntry,
};

use super::*;

const OWNER_REGISTRY: &str = "minecraft:enchantment";
const OWNER_CODEC: &str = "Enchantment.DIRECT_CODEC";

fn context(source: RegistryElementSource) -> RegistryCodecDecodeContext<'static> {
    RegistryCodecDecodeContext::new(OWNER_REGISTRY, OWNER_CODEC, source)
}

fn network(value: &NbtSummaryValue) -> RegistryElementData {
    decode_network_registry_element(&encode_nbt_summary(value, 1 << 20).unwrap()).unwrap()
}

fn decode(data: &RegistryElementData) -> LootContextCodecValue {
    decode_number_provider_source(data.root(), "$", &mut context(data.source)).unwrap()
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

fn float(value: f32) -> NbtSummaryValue {
    NbtSummaryValue::Float(canonical_float_tag_bits(value))
}

fn provider(kind: &str, fields: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    let mut fields = fields;
    fields.push(entry("type", string(&format!("minecraft:{kind}"))));
    compound(fields)
}

fn assert_all_sources(
    json: &str,
    network_input: &NbtSummaryValue,
    expected: &NbtSummaryValue,
    expected_params: &[VanillaLootContextParam],
) {
    for data in [
        decode_direct_registry_json(json.as_bytes()).unwrap(),
        decode_client_known_pack_registry_json(json.as_bytes()).unwrap(),
        network(network_input),
    ] {
        let decoded = decode(&data);
        assert_eq!(
            decoded.canonical.network_nbt_unchecked(),
            expected,
            "{json}"
        );
        assert!(decoded.canonical.references().next().is_none(), "{json}");
        assert_eq!(decoded.referenced_context_params, expected_params, "{json}");
    }
}

#[test]
fn all_eight_families_have_exact_canonical_trees_in_all_three_sources() {
    let this_param = vanilla_loot_entity_target("this").unwrap().context_param;
    let cases = vec![
        (
            r#"{"type":"constant","value":2.5}"#,
            compound(vec![
                entry("type", string("constant")),
                entry("value", float(2.5)),
            ]),
            float(2.5),
            vec![],
        ),
        (
            r#"{"type":"uniform","min":1,"max":2}"#,
            compound(vec![
                entry("type", string("uniform")),
                entry("min", float(1.0)),
                entry("max", float(2.0)),
            ]),
            provider(
                "uniform",
                vec![entry("min", float(1.0)), entry("max", float(2.0))],
            ),
            vec![],
        ),
        (
            r#"{"type":"binomial","n":3,"p":0.25}"#,
            compound(vec![
                entry("type", string("binomial")),
                entry("n", float(3.0)),
                entry("p", float(0.25)),
            ]),
            provider(
                "binomial",
                vec![entry("n", float(3.0)), entry("p", float(0.25))],
            ),
            vec![],
        ),
        (
            r#"{"type":"score","target":"this","score":"kills","scale":2}"#,
            compound(vec![
                entry("type", string("score")),
                entry("target", string("this")),
                entry("score", string("kills")),
                entry("scale", float(2.0)),
            ]),
            provider(
                "score",
                vec![
                    entry("target", string("this")),
                    entry("score", string("kills")),
                    entry("scale", float(2.0)),
                ],
            ),
            vec![this_param],
        ),
        (
            r#"{"type":"storage","storage":"numbers","path":"foo[{id:'pig'}].bar trailing"}"#,
            compound(vec![
                entry("type", string("storage")),
                entry("storage", string("numbers")),
                entry("path", string("foo[{id:'pig'}].bar trailing")),
            ]),
            provider(
                "storage",
                vec![
                    entry("storage", string("minecraft:numbers")),
                    entry("path", string("foo[{id:'pig'}].bar")),
                ],
            ),
            vec![],
        ),
        (
            r#"{"type":"sum","summands":[]}"#,
            compound(vec![
                entry("type", string("sum")),
                entry("summands", list(vec![])),
            ]),
            provider("sum", vec![entry("summands", list(vec![]))]),
            vec![],
        ),
        (
            r#"{"type":"enchantment_level","amount":{"type":"linear","base":1,"per_level_above_first":2}}"#,
            compound(vec![
                entry("type", string("enchantment_level")),
                entry(
                    "amount",
                    compound(vec![
                        entry("type", string("linear")),
                        entry("base", float(1.0)),
                        entry("per_level_above_first", float(2.0)),
                    ]),
                ),
            ]),
            provider(
                "enchantment_level",
                vec![entry(
                    "amount",
                    compound(vec![
                        entry("base", float(1.0)),
                        entry("per_level_above_first", float(2.0)),
                        entry("type", string("minecraft:linear")),
                    ]),
                )],
            ),
            vec![],
        ),
        (
            r#"{"type":"environment_attribute","attribute":"visual/fog_start_distance"}"#,
            compound(vec![
                entry("type", string("environment_attribute")),
                entry("attribute", string("visual/fog_start_distance")),
            ]),
            provider(
                "environment_attribute",
                vec![entry(
                    "attribute",
                    string("minecraft:visual/fog_start_distance"),
                )],
            ),
            vec![VanillaLootContextParam::Origin],
        ),
    ];

    for (json, network_input, expected, params) in cases {
        assert_all_sources(json, &network_input, &expected, &params);
    }
}

#[test]
fn inline_and_typed_constants_and_untyped_uniform_use_writer_alternatives() {
    assert_all_sources(
        "-0.0",
        &NbtSummaryValue::Float((-0.0_f32).to_bits()),
        &float(0.0),
        &[],
    );
    assert_all_sources(
        r#"{"value":4,"type":"minecraft:constant"}"#,
        &compound(vec![
            entry("value", float(4.0)),
            entry("type", string("minecraft:constant")),
        ]),
        &float(4.0),
        &[],
    );
    assert_all_sources(
        r#"{"min":1,"max":{"type":"constant","value":3}}"#,
        &compound(vec![
            entry("min", float(1.0)),
            entry(
                "max",
                compound(vec![
                    entry("type", string("constant")),
                    entry("value", float(3.0)),
                ]),
            ),
        ]),
        &provider(
            "uniform",
            vec![entry("min", float(1.0)), entry("max", float(3.0))],
        ),
        &[],
    );
}

#[test]
fn score_default_scale_is_always_written_and_target_facts_survive_collapse() {
    let target_param = vanilla_loot_entity_target("attacker")
        .unwrap()
        .context_param;
    let input = compound(vec![
        entry("type", string("score")),
        entry(
            "target",
            compound(vec![
                entry("type", string("context")),
                entry("target", string("attacker")),
            ]),
        ),
        entry("score", string("damage")),
    ]);
    let expected = provider(
        "score",
        vec![
            entry("target", string("attacker")),
            entry("score", string("damage")),
            entry("scale", float(1.0)),
        ],
    );
    assert_all_sources(
        r#"{"type":"score","target":{"type":"context","target":"attacker"},"score":"damage"}"#,
        &input,
        &expected,
        &[target_param],
    );
}

#[test]
fn storage_uses_nbt_path_consumed_prefix_and_identifier_normalization() {
    for (raw, expected) in [
        ("", ""),
        (" foo", ""),
        ("foo bar", "foo"),
        ("foo[{id:'pig'}].bar trailing", "foo[{id:'pig'}].bar"),
    ] {
        let json = format!(
            "{{\"type\":\"storage\",\"storage\":\"example\",\"path\":{}}}",
            serde_json::to_string(raw).unwrap()
        );
        let decoded = decode(&decode_direct_registry_json(json.as_bytes()).unwrap());
        assert_eq!(
            decoded.canonical.network_nbt_unchecked(),
            &provider(
                "storage",
                vec![
                    entry("storage", string("minecraft:example")),
                    entry("path", string(expected)),
                ],
            ),
            "{raw:?}"
        );
    }

    for json in [
        r#"{"type":"storage","storage":"Upper:bad","path":"foo"}"#,
        r#"{"type":"storage","storage":"ok","path":"["}"#,
        r#"{"type":"storage","storage":1,"path":"foo"}"#,
        r#"{"type":"storage","storage":"ok","path":1}"#,
    ] {
        let data = decode_direct_registry_json(json.as_bytes()).unwrap();
        assert!(
            decode_number_provider_source(data.root(), "$", &mut context(data.source)).is_err(),
            "{json}"
        );
    }
}
