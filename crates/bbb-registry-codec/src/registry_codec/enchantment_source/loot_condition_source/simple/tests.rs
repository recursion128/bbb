use super::*;
use crate::{
    decode_client_known_pack_registry_json, decode_direct_registry_json,
    decode_network_registry_element,
    registry_codec::{
        recursive_values::RegistryCodecDecodeContext, RegistryElementData, RegistryElementSource,
    },
};
use bbb_protocol::{encode_nbt_summary, nbt_summary::entry};

const OWNER_REGISTRY: &str = "minecraft:enchantment";
const OWNER_CODEC: &str = "Enchantment.DIRECT_CODEC";

fn context(source: RegistryElementSource) -> RegistryCodecDecodeContext<'static> {
    RegistryCodecDecodeContext::new(OWNER_REGISTRY, OWNER_CODEC, source)
}

fn decode(data: &RegistryElementData) -> LootContextCodecValue {
    decode_simple_condition_source(data.root(), "$", &mut context(data.source))
        .expect("test condition belongs to simple owner")
        .unwrap()
}

fn network(value: &NbtSummaryValue) -> RegistryElementData {
    decode_network_registry_element(&encode_nbt_summary(value, 1 << 20).unwrap()).unwrap()
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

fn float_value(value: f32) -> NbtSummaryValue {
    float(value)
}

fn condition(kind: &str, mut fields: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    fields.push(entry("condition", string(&format!("minecraft:{kind}"))));
    compound(fields)
}

fn assert_all_sources(
    json: &str,
    network_input: &NbtSummaryValue,
    expected: &NbtSummaryValue,
    params: &[VanillaLootContextParam],
    references: &[(&str, &str)],
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
        assert_eq!(decoded.referenced_context_params, params, "{json}");
        assert_eq!(
            decoded
                .canonical
                .references()
                .map(|reference| (reference.target_registry, reference.resource_id.as_str()))
                .collect::<Vec<_>>(),
            references,
            "{json}"
        );
    }
}

#[test]
fn all_ten_families_have_exact_three_source_trees_references_and_validation_facts() {
    let cases = vec![
        (
            r#"{"condition":"random_chance","chance":{"type":"constant","value":0.25},"ignored":7}"#,
            condition(
                "random_chance",
                vec![
                    entry("chance", float_value(0.25)),
                    entry("ignored", NbtSummaryValue::Int(7)),
                ],
            ),
            condition("random_chance", vec![entry("chance", float_value(0.25))]),
            vec![],
            vec![],
        ),
        (
            r#"{"condition":"random_chance_with_enchanted_bonus","unenchanted_chance":0.25,"enchanted_chance":{"type":"linear","base":0.5,"per_level_above_first":0.1},"enchantment":"looting"}"#,
            condition(
                "random_chance_with_enchanted_bonus",
                vec![
                    entry("unenchanted_chance", float_value(0.25)),
                    entry(
                        "enchanted_chance",
                        compound(vec![
                            entry("base", float_value(0.5)),
                            entry("per_level_above_first", float_value(0.1)),
                            entry("type", string("linear")),
                        ]),
                    ),
                    entry("enchantment", string("looting")),
                ],
            ),
            condition(
                "random_chance_with_enchanted_bonus",
                vec![
                    entry("unenchanted_chance", float_value(0.25)),
                    entry(
                        "enchanted_chance",
                        compound(vec![
                            entry("base", float_value(0.5)),
                            entry("per_level_above_first", float_value(0.1)),
                            entry("type", string("minecraft:linear")),
                        ]),
                    ),
                    entry("enchantment", string("minecraft:looting")),
                ],
            ),
            vec![VanillaLootContextParam::AttackingEntity],
            vec![(ENCHANTMENT_REGISTRY, "minecraft:looting")],
        ),
        (
            r#"{"condition":"killed_by_player","ignored":true}"#,
            condition(
                "killed_by_player",
                vec![entry("ignored", NbtSummaryValue::Byte(1))],
            ),
            condition("killed_by_player", vec![]),
            vec![VanillaLootContextParam::LastDamagePlayer],
            vec![],
        ),
        (
            r#"{"condition":"entity_scores","scores":{"kills":{"min":{"type":"score","target":"attacker","score":"nested"}},"deaths":2},"entity":"this"}"#,
            condition(
                "entity_scores",
                vec![
                    entry(
                        "scores",
                        compound(vec![
                            entry(
                                "kills",
                                compound(vec![entry(
                                    "min",
                                    compound(vec![
                                        entry("target", string("attacker")),
                                        entry("score", string("nested")),
                                        entry("type", string("minecraft:score")),
                                    ]),
                                )]),
                            ),
                            entry("deaths", NbtSummaryValue::Int(2)),
                        ]),
                    ),
                    entry("entity", string("this")),
                ],
            ),
            condition(
                "entity_scores",
                vec![
                    entry(
                        "scores",
                        compound(vec![
                            entry(
                                "kills",
                                compound(vec![entry(
                                    "min",
                                    compound(vec![
                                        entry("target", string("attacker")),
                                        entry("score", string("nested")),
                                        entry("scale", float_value(1.0)),
                                        entry("type", string("minecraft:score")),
                                    ]),
                                )]),
                            ),
                            entry("deaths", NbtSummaryValue::Int(2)),
                        ]),
                    ),
                    entry("entity", string("this")),
                ],
            ),
            vec![
                VanillaLootContextParam::ThisEntity,
                VanillaLootContextParam::AttackingEntity,
            ],
            vec![],
        ),
        (
            r#"{"condition":"table_bonus","enchantment":"fortune","chances":[0.1,1]}"#,
            condition(
                "table_bonus",
                vec![
                    entry("enchantment", string("fortune")),
                    entry("chances", list(vec![float_value(0.1), float_value(1.0)])),
                ],
            ),
            condition(
                "table_bonus",
                vec![
                    entry("enchantment", string("minecraft:fortune")),
                    entry("chances", list(vec![float_value(0.1), float_value(1.0)])),
                ],
            ),
            vec![VanillaLootContextParam::Tool],
            vec![(ENCHANTMENT_REGISTRY, "minecraft:fortune")],
        ),
        (
            r#"{"condition":"survives_explosion"}"#,
            condition("survives_explosion", vec![]),
            condition("survives_explosion", vec![]),
            vec![VanillaLootContextParam::ExplosionRadius],
            vec![],
        ),
        (
            r#"{"condition":"weather_check","raining":false,"thundering":true}"#,
            condition(
                "weather_check",
                vec![
                    entry("raining", NbtSummaryValue::Byte(0)),
                    entry("thundering", NbtSummaryValue::Byte(1)),
                ],
            ),
            condition(
                "weather_check",
                vec![
                    entry("raining", NbtSummaryValue::Byte(0)),
                    entry("thundering", NbtSummaryValue::Byte(1)),
                ],
            ),
            vec![],
            vec![],
        ),
        (
            r#"{"condition":"time_check","clock":"day","period":24000,"value":{"min":{"type":"environment_attribute","attribute":"visual/fog_start_distance"},"max":10}}"#,
            condition(
                "time_check",
                vec![
                    entry("clock", string("day")),
                    entry("period", NbtSummaryValue::Long(24_000)),
                    entry(
                        "value",
                        compound(vec![
                            entry(
                                "min",
                                compound(vec![
                                    entry("attribute", string("visual/fog_start_distance")),
                                    entry("type", string("minecraft:environment_attribute")),
                                ]),
                            ),
                            entry("max", float_value(10.0)),
                        ]),
                    ),
                ],
            ),
            condition(
                "time_check",
                vec![
                    entry("clock", string("minecraft:day")),
                    entry("period", NbtSummaryValue::Long(24_000)),
                    entry(
                        "value",
                        compound(vec![
                            entry(
                                "min",
                                compound(vec![
                                    entry(
                                        "attribute",
                                        string("minecraft:visual/fog_start_distance"),
                                    ),
                                    entry("type", string("minecraft:environment_attribute")),
                                ]),
                            ),
                            entry("max", float_value(10.0)),
                        ]),
                    ),
                ],
            ),
            vec![VanillaLootContextParam::Origin],
            vec![(WORLD_CLOCK_REGISTRY, "minecraft:day")],
        ),
        (
            r#"{"condition":"value_check","value":{"type":"score","target":"target_entity","score":"kills"},"range":{"min":{"type":"environment_attribute","attribute":"visual/fog_start_distance"}}}"#,
            condition(
                "value_check",
                vec![
                    entry(
                        "value",
                        compound(vec![
                            entry("target", string("target_entity")),
                            entry("score", string("kills")),
                            entry("type", string("minecraft:score")),
                        ]),
                    ),
                    entry(
                        "range",
                        compound(vec![entry(
                            "min",
                            compound(vec![
                                entry("attribute", string("visual/fog_start_distance")),
                                entry("type", string("minecraft:environment_attribute")),
                            ]),
                        )]),
                    ),
                ],
            ),
            condition(
                "value_check",
                vec![
                    entry(
                        "value",
                        compound(vec![
                            entry("target", string("target_entity")),
                            entry("score", string("kills")),
                            entry("scale", float_value(1.0)),
                            entry("type", string("minecraft:score")),
                        ]),
                    ),
                    entry(
                        "range",
                        compound(vec![entry(
                            "min",
                            compound(vec![
                                entry("attribute", string("minecraft:visual/fog_start_distance")),
                                entry("type", string("minecraft:environment_attribute")),
                            ]),
                        )]),
                    ),
                ],
            ),
            vec![
                VanillaLootContextParam::TargetEntity,
                VanillaLootContextParam::Origin,
            ],
            vec![],
        ),
        (
            r#"{"condition":"enchantment_active_check","active":false}"#,
            condition(
                "enchantment_active_check",
                vec![entry("active", NbtSummaryValue::Byte(0))],
            ),
            condition(
                "enchantment_active_check",
                vec![entry("active", NbtSummaryValue::Byte(0))],
            ),
            vec![VanillaLootContextParam::EnchantmentActive],
            vec![],
        ),
    ];

    for (json, network_input, expected, params, references) in cases {
        assert_all_sources(json, &network_input, &expected, &params, &references);
    }
}

#[test]
fn defaults_unit_writers_and_condition_last_are_exact() {
    let weather =
        decode(&decode_direct_registry_json(br#"{"condition":"weather_check"}"#).unwrap());
    assert_eq!(
        weather.canonical.network_nbt_unchecked(),
        &condition("weather_check", vec![])
    );
    for kind in ["killed_by_player", "survives_explosion"] {
        let json = format!(r#"{{"condition":"{kind}","unknown":1}}"#);
        let value = decode(&decode_direct_registry_json(json.as_bytes()).unwrap());
        assert_eq!(
            value.canonical.network_nbt_unchecked(),
            &condition(kind, vec![])
        );
    }
    let time = decode(
        &decode_direct_registry_json(br#"{"condition":"time_check","clock":"day","value":4}"#)
            .unwrap(),
    );
    assert_eq!(
        time.canonical.network_nbt_unchecked(),
        &condition(
            "time_check",
            vec![
                entry("clock", string("minecraft:day")),
                entry("value", NbtSummaryValue::Int(4)),
            ],
        )
    );
}
