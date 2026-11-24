use std::collections::HashSet;

use crate::registry_codec::{
    decode_client_known_pack_registry_json, decode_direct_registry_json,
    recursive_values::RegistryCodecDecodeContext, RegistryElementData, RegistryElementSource,
};
use bbb_protocol::{nbt_summary::canonical_float_tag_bits, NbtSummaryEntry, NbtSummaryValue};

use super::*;

const OWNER_REGISTRY: &str = "minecraft:enchantment";
const OWNER_CODEC: &str = "Enchantment.DIRECT_CODEC";

fn context(source: RegistryElementSource) -> RegistryCodecDecodeContext<'static> {
    RegistryCodecDecodeContext::new(OWNER_REGISTRY, OWNER_CODEC, source)
}

fn decode(data: &RegistryElementData) -> CanonicalRegistryCodecValue {
    decode_enchantment_effect_components_source(data.root(), "$.effects", &mut context(data.source))
        .unwrap()
}

fn decode_json(json: &str) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let data = decode_client_known_pack_registry_json(json.as_bytes()).unwrap();
    decode_enchantment_effect_components_source(data.root(), "$.effects", &mut context(data.source))
}

fn network(value: &NbtSummaryValue) -> RegistryElementData {
    RegistryElementData::from_network_nbt(value.clone())
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
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

fn float(value: f32) -> NbtSummaryValue {
    NbtSummaryValue::Float(canonical_float_tag_bits(value))
}

fn add_effect() -> NbtSummaryValue {
    compound(vec![
        entry("value", float(1.0)),
        entry("type", string("minecraft:add")),
    ])
}

fn change_damage_effect() -> NbtSummaryValue {
    compound(vec![
        entry("amount", float(1.0)),
        entry("type", string("minecraft:change_item_damage")),
    ])
}

fn one_component(id: &str, value: NbtSummaryValue) -> NbtSummaryValue {
    compound(vec![entry(id, value)])
}

fn family_cases() -> Vec<(&'static str, &'static str, NbtSummaryValue)> {
    vec![
        (
            "damage",
            r#"[{"requirements":null,"effect":{"type":"add","value":1}}]"#,
            list(vec![compound(vec![entry("effect", add_effect())])]),
        ),
        (
            "damage_immunity",
            r#"[{"effect":{}}]"#,
            list(vec![compound(vec![entry("effect", compound(vec![]))])]),
        ),
        (
            "post_attack",
            r#"[{"effect":{"type":"change_item_damage","amount":1},"affected":"damaging_entity","enchanted":"attacker"}]"#,
            list(vec![compound(vec![
                entry("enchanted", string("attacker")),
                entry("affected", string("damaging_entity")),
                entry("effect", change_damage_effect()),
            ])]),
        ),
        (
            "post_piercing_attack",
            r#"[{"effect":{"type":"change_item_damage","amount":1}}]"#,
            list(vec![compound(vec![entry(
                "effect",
                change_damage_effect(),
            )])]),
        ),
        (
            "equipment_drops",
            r#"[{"affected":"attacker","effect":{"type":"add","value":1},"enchanted":"victim"}]"#,
            list(vec![compound(vec![
                entry("enchanted", string("victim")),
                entry("effect", add_effect()),
            ])]),
        ),
        (
            "location_changed",
            r#"[{"effect":{"type":"change_item_damage","amount":1}}]"#,
            list(vec![compound(vec![entry(
                "effect",
                change_damage_effect(),
            )])]),
        ),
        (
            "attributes",
            r#"[{"operation":"add_value","amount":1,"attribute":"example:attribute","id":"test"}]"#,
            list(vec![compound(vec![
                entry("id", string("minecraft:test")),
                entry("attribute", string("example:attribute")),
                entry("amount", float(1.0)),
                entry("operation", string("add_value")),
            ])]),
        ),
        (
            "crossbow_charge_time",
            r#"{"type":"add","value":1}"#,
            add_effect(),
        ),
        (
            "crossbow_charging_sounds",
            r#"[{"end":"example:end","start":"example:start"}]"#,
            list(vec![compound(vec![
                entry("start", string("example:start")),
                entry("end", string("example:end")),
            ])]),
        ),
        (
            "trident_sound",
            r#"["example:first","example:second"]"#,
            list(vec![string("example:first"), string("example:second")]),
        ),
        (
            "prevent_equipment_drop",
            r#"{"ignored":1}"#,
            compound(vec![]),
        ),
    ]
}

#[test]
fn all_eleven_families_write_exact_trees_for_all_three_sources() {
    let mut families = HashSet::new();
    for (id, payload, value) in family_cases() {
        let descriptor = enchantment_effect_component_codec_type(&format!("minecraft:{id}"))
            .expect("family test names a generated component");
        assert!(families.insert(descriptor.codec_family));
        let json = format!(r#"{{"{id}":{payload}}}"#);
        let expected = one_component(&format!("minecraft:{id}"), value);
        for data in [
            decode_direct_registry_json(json.as_bytes()).unwrap(),
            decode_client_known_pack_registry_json(json.as_bytes()).unwrap(),
            network(&expected),
        ] {
            assert_eq!(
                decode(&data).network_nbt_unchecked(),
                &expected,
                "{id} {:?}",
                data.source
            );
        }
    }
    assert_eq!(families.len(), 11);
}

#[test]
fn nested_requirements_check_context_usage_and_forbid_predicate_references() {
    for (target, accepted) in [("this", true), ("interacting_entity", false)] {
        let json = format!(
            r#"{{"damage_immunity":[{{"effect":{{}},"requirements":{{"condition":"all_of","terms":[{{"condition":"inverted","term":{{"condition":"value_check","value":0,"range":{{"min":{{"type":"score","target":"{target}","score":"kills"}}}}}}}}]}}}}]}}"#
        );
        assert_eq!(decode_json(&json).is_ok(), accepted, "{target}");
    }
    assert!(decode_json(
        r#"{"damage_immunity":[{"effect":{},"requirements":{"condition":"inverted","term":[{"condition":"reference","name":"example:predicate"}]}}]}"#,
    )
    .is_err());
}

#[test]
fn targeted_and_equipment_drop_enums_follow_their_distinct_codecs() {
    assert!(decode_json(
        r#"{"post_attack":[{"enchanted":"damaging_entity","affected":"victim","effect":{"type":"change_item_damage","amount":1}}]}"#,
    )
    .is_ok());

    let bad_target = decode_json(
        r#"{"post_attack":[{"enchanted":"other","affected":"victim","effect":{"type":"change_item_damage","amount":1}}]}"#,
    )
    .unwrap_err();
    assert_eq!(bad_target.field, "$.effects.post_attack[0].enchanted");

    let bad_equipment = decode_json(
        r#"{"equipment_drops":[{"enchanted":"damaging_entity","effect":{"type":"add","value":1}}]}"#,
    )
    .unwrap_err();
    assert!(bad_equipment.reason.contains("non-damage"));

    let accepted = decode_json(
        r#"{"equipment_drops":[{"affected":"attacker","enchanted":"victim","effect":{"type":"add","value":1}}]}"#,
    )
    .unwrap();
    assert_eq!(
        accepted.network_nbt_unchecked(),
        &one_component(
            "minecraft:equipment_drops",
            list(vec![compound(vec![
                entry("enchanted", string("victim")),
                entry("effect", add_effect()),
            ])]),
        )
    );
}
