use crate::{
    decode_client_known_pack_registry_json, decode_direct_registry_json,
    decode_network_registry_element,
    loot_context_types::VanillaLootContextParam,
    registry_codec::{
        recursive_values::RegistryCodecDecodeContext, RegistryElementData, RegistryReferenceKind,
    },
};
use bbb_protocol::{encode_nbt_summary, NbtSummaryEntry, NbtSummaryValue};

use super::super::LootConditionCodecValue;

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn compound(fields: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(fields)
}

fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::String(value.to_string().into())
}

fn network(value: &NbtSummaryValue) -> RegistryElementData {
    decode_network_registry_element(&encode_nbt_summary(value, 1 << 20).unwrap()).unwrap()
}

fn decode(data: &RegistryElementData) -> LootConditionCodecValue {
    let mut context = RegistryCodecDecodeContext::new(
        "minecraft:enchantment",
        "LootItemCondition.DIRECT_CODEC",
        data.source,
    );
    super::super::decode_loot_condition_source(data.root(), "$", &mut context).unwrap()
}

#[test]
fn predicate_backed_families_write_full_canonical_trees_for_three_sources() {
    let cases = [
        (
            br#"{"condition":"entity_properties","predicate":{},"entity":"attacker"}"#.as_slice(),
            compound(vec![
                entry("predicate", compound(vec![])),
                entry("entity", string("attacker")),
                entry("condition", string("minecraft:entity_properties")),
            ]),
            vec![VanillaLootContextParam::Origin, VanillaLootContextParam::AttackingEntity],
        ),
        (
            br#"{"condition":"block_state_property","block":"oak_log","properties":{"axis":"x"}}"#.as_slice(),
            compound(vec![
                entry("block", string("minecraft:oak_log")),
                entry("properties", compound(vec![entry("axis", string("x"))])),
                entry("condition", string("minecraft:block_state_property")),
            ]),
            vec![VanillaLootContextParam::BlockState],
        ),
        (
            br#"{"condition":"match_tool","predicate":{}}"#.as_slice(),
            compound(vec![
                entry("predicate", compound(vec![])),
                entry("condition", string("minecraft:match_tool")),
            ]),
            vec![VanillaLootContextParam::Tool],
        ),
        (
            br#"{"condition":"damage_source_properties","predicate":{}}"#.as_slice(),
            compound(vec![
                entry("predicate", compound(vec![])),
                entry("condition", string("minecraft:damage_source_properties")),
            ]),
            vec![VanillaLootContextParam::Origin, VanillaLootContextParam::DamageSource],
        ),
        (
            br#"{"condition":"location_check","predicate":{},"offsetX":1,"offsetY":0,"offsetZ":-2}"#.as_slice(),
            compound(vec![
                entry("predicate", compound(vec![])),
                entry("offsetX", NbtSummaryValue::Int(1)),
                entry("offsetZ", NbtSummaryValue::Int(-2)),
                entry("condition", string("minecraft:location_check")),
            ]),
            vec![VanillaLootContextParam::Origin],
        ),
        (
            br#"{"condition":"environment_attribute_check","attribute":"gameplay/fast_lava","value":true}"#.as_slice(),
            compound(vec![
                entry("value", NbtSummaryValue::Byte(1)),
                entry("attribute", string("minecraft:gameplay/fast_lava")),
                entry("condition", string("minecraft:environment_attribute_check")),
            ]),
            vec![],
        ),
    ];

    for (json, expected, params) in cases {
        for data in [
            decode_direct_registry_json(json).unwrap(),
            decode_client_known_pack_registry_json(json).unwrap(),
            network(&expected),
        ] {
            let decoded = decode(&data);
            assert_eq!(decoded.value.canonical.network_nbt_unchecked(), &expected);
            assert_eq!(decoded.value.referenced_context_params, params);
        }
    }
}

#[test]
fn fixed_block_and_item_holders_retain_generation_references() {
    let block = decode(
        &decode_direct_registry_json(br#"{"condition":"block_state_property","block":"oak_log"}"#)
            .unwrap(),
    );
    let refs = block.value.canonical.references().collect::<Vec<_>>();
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].target_registry, "minecraft:block");
    assert_eq!(refs[0].kind, RegistryReferenceKind::Element);
    assert_eq!(refs[0].resource_id.as_str(), "minecraft:oak_log");

    let item = decode(
        &decode_direct_registry_json(
            br#"{"condition":"match_tool","predicate":{"items":"stone"}}"#,
        )
        .unwrap(),
    );
    let refs = item.value.canonical.references().collect::<Vec<_>>();
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].target_registry, "minecraft:item");
    assert_eq!(refs[0].resource_id.as_str(), "minecraft:stone");
}
