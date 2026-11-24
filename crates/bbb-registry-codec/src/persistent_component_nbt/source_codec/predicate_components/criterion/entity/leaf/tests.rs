use crate::{
    decode_client_known_pack_registry_json, decode_direct_registry_json,
    decode_network_registry_element,
    registry_codec::{
        recursive_values::{CanonicalRegistryCodecValue, RegistryCodecDecodeContext},
        RegistryCodecError, RegistryElementData, RegistryElementSource, RegistryReferenceKind,
    },
};
use bbb_facts::stat_types::VANILLA_STAT_TYPES;
use bbb_protocol::{
    encode_nbt_summary, nbt_summary::canonical_double_tag_bits, NbtSummaryEntry, NbtSummaryValue,
};

use super::super::super::super::RegistryCodecRecord;
use super::*;

const OWNER: &str = "minecraft:advancement";
const CODEC: &str = "EntityPredicate.CODEC";

fn context(source: RegistryElementSource) -> RegistryCodecDecodeContext<'static> {
    RegistryCodecDecodeContext::new(OWNER, CODEC, source)
}

fn network(value: &NbtSummaryValue) -> RegistryElementData {
    decode_network_registry_element(&encode_nbt_summary(value, 1 << 20).unwrap()).unwrap()
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

fn double(value: f64) -> NbtSummaryValue {
    NbtSummaryValue::Double(canonical_double_tag_bits(value))
}

fn decode_leaf_bundle(
    data: &RegistryElementData,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let input = data.root();
    let field = "$";
    let mut context = context(data.source);
    context.charge_record(input, field, "entity leaf bundle")?;
    let mut output = RegistryCodecRecord::default();
    macro_rules! push {
        ($name:literal, $decode:expr) => {
            if let Some(value) = input.field($name) {
                let nested = child(field, $name);
                let value =
                    context.with_nested(&nested, |context| $decode(value, &nested, context))?;
                output.push($name, value);
            }
        };
    }
    push!("type", decode_entity_type_predicate);
    push!("distance", decode_distance_predicate);
    push!("movement", decode_movement_predicate);
    push!("effects", decode_mob_effects_predicate);
    push!("nbt", decode_entity_nbt_predicate);
    push!("flags", decode_entity_flags_predicate);
    if let Some(value) = input.field("equipment") {
        let nested = child(field, "equipment");
        let value = context.with_nested(&nested, |context| {
            decode_entity_equipment_predicate(value, &nested, context)
        })?;
        output.push("equipment", value);
    }
    if let Some(value) = input.field("slots") {
        let nested = child(field, "slots");
        let value = context.with_nested(&nested, |context| {
            decode_slots_predicate(value, &nested, context)
        })?;
        output.push("slots", value);
    }
    if let Some(value) = input.field("food") {
        let nested = child(field, "food");
        let value = context.with_nested(&nested, |context| {
            decode_food_predicate(value, &nested, context)
        })?;
        output.push("food", value.value);
    }
    if let Some(value) = input.field("gamemode") {
        let nested = child(field, "gamemode");
        let value = context.with_nested(&nested, |context| {
            decode_game_type_predicate(value, &nested, context)
        })?;
        output.push("gamemode", value.value);
    }
    push!("stats", decode_player_stats);
    push!("recipes", decode_player_recipes);
    push!("advancements", decode_player_advancements);
    push!("input", decode_input_predicate);
    Ok(output.finish())
}

#[test]
fn criterion_entity_leaf_three_sources_write_exact_complete_tree() {
    let json = br##"{
      "input":{"sprint":false,"forward":true},
      "advancements":{"story/root":true,"adventure/test":{"first":false}},
      "recipes":{"bread":true},
      "stats":[{"type":"mined","stat":"stone","value":{"min":2}}],
      "gamemode":["survival","adventure"],
      "food":{"level":{"min":10},"saturation":2.5},
      "slots":{"armor.*":{"items":"diamond_helmet"}},
      "equipment":{"head":{"items":"diamond_helmet"},"mainhand":{"count":1}},
      "flags":{"is_on_ground":true,"is_flying":false},
      "nbt":"{Health:20f}",
      "effects":{"speed":{"amplifier":{"min":1},"visible":true}},
      "movement":{"x":-0.0,"speed":{"max":4.0}},
      "distance":{"x":1.5,"horizontal":{"min":2.0}},
      "type":"#skeletons"
    }"##;
    let expected = compound(vec![
        entry("type", string("#minecraft:skeletons")),
        entry(
            "distance",
            compound(vec![
                entry("x", double(1.5)),
                entry("horizontal", compound(vec![entry("min", double(2.0))])),
            ]),
        ),
        entry(
            "movement",
            compound(vec![
                entry("x", double(0.0)),
                entry("speed", compound(vec![entry("max", double(4.0))])),
            ]),
        ),
        entry(
            "effects",
            compound(vec![entry(
                "minecraft:speed",
                compound(vec![
                    entry(
                        "amplifier",
                        compound(vec![entry("min", NbtSummaryValue::Int(1))]),
                    ),
                    entry("visible", NbtSummaryValue::Byte(1)),
                ]),
            )]),
        ),
        entry("nbt", string("{Health:20.0f}")),
        entry(
            "flags",
            compound(vec![
                entry("is_on_ground", NbtSummaryValue::Byte(1)),
                entry("is_flying", NbtSummaryValue::Byte(0)),
            ]),
        ),
        entry(
            "equipment",
            compound(vec![
                entry(
                    "head",
                    compound(vec![entry("items", string("minecraft:diamond_helmet"))]),
                ),
                entry(
                    "mainhand",
                    compound(vec![entry("count", NbtSummaryValue::Int(1))]),
                ),
            ]),
        ),
        entry(
            "slots",
            compound(vec![entry(
                "armor.*",
                compound(vec![entry("items", string("minecraft:diamond_helmet"))]),
            )]),
        ),
        entry(
            "food",
            compound(vec![
                entry(
                    "level",
                    compound(vec![entry("min", NbtSummaryValue::Int(10))]),
                ),
                entry("saturation", double(2.5)),
            ]),
        ),
        entry(
            "gamemode",
            list(vec![string("survival"), string("adventure")]),
        ),
        entry(
            "stats",
            list(vec![compound(vec![
                entry("stat", string("minecraft:stone")),
                entry(
                    "value",
                    compound(vec![entry("min", NbtSummaryValue::Int(2))]),
                ),
                entry("type", string("minecraft:mined")),
            ])]),
        ),
        entry(
            "recipes",
            compound(vec![entry("minecraft:bread", NbtSummaryValue::Byte(1))]),
        ),
        entry(
            "advancements",
            compound(vec![
                entry("minecraft:story/root", NbtSummaryValue::Byte(1)),
                entry(
                    "minecraft:adventure/test",
                    compound(vec![entry("first", NbtSummaryValue::Byte(0))]),
                ),
            ]),
        ),
        entry(
            "input",
            compound(vec![
                entry("forward", NbtSummaryValue::Byte(1)),
                entry("sprint", NbtSummaryValue::Byte(0)),
            ]),
        ),
    ]);

    for data in [
        decode_direct_registry_json(json).unwrap(),
        decode_client_known_pack_registry_json(json).unwrap(),
        network(&expected),
    ] {
        let decoded = decode_leaf_bundle(&data).unwrap();
        assert_eq!(decoded.network_nbt_unchecked(), &expected);
        let refs = decoded.references().collect::<Vec<_>>();
        assert_eq!(refs.len(), 5);
        assert_eq!(refs[0].target_registry, ENTITY_TYPE_REGISTRY);
        assert_eq!(refs[0].kind, RegistryReferenceKind::Tag);
        assert_eq!(refs[0].resource_id.as_str(), "minecraft:skeletons");
        assert_eq!(refs[1].target_registry, MOB_EFFECT_REGISTRY);
        assert_eq!(refs[1].kind, RegistryReferenceKind::Element);
        assert_eq!(refs[1].resource_id.as_str(), "minecraft:speed");
        assert_eq!(refs[2].target_registry, "minecraft:item");
        assert_eq!(refs[2].resource_id.as_str(), "minecraft:diamond_helmet");
        assert_eq!(refs[3].target_registry, "minecraft:item");
        assert_eq!(refs[3].resource_id.as_str(), "minecraft:diamond_helmet");
        assert_eq!(refs[4].target_registry, "minecraft:block");
        assert_eq!(refs[4].resource_id.as_str(), "minecraft:stone");
    }
}

#[test]
fn criterion_entity_leaf_effects_three_sources_references_aliases_and_default_instance() {
    let json = br#"{"speed":{},"strength":{"duration":20,"ambient":false}}"#;
    let expected = compound(vec![
        entry("minecraft:speed", compound(vec![])),
        entry(
            "minecraft:strength",
            compound(vec![
                entry("duration", NbtSummaryValue::Int(20)),
                entry("ambient", NbtSummaryValue::Byte(0)),
            ]),
        ),
    ]);
    for data in [
        decode_direct_registry_json(json).unwrap(),
        decode_client_known_pack_registry_json(json).unwrap(),
        network(&expected),
    ] {
        let decoded =
            decode_mob_effects_predicate(data.root(), "$", &mut context(data.source)).unwrap();
        assert_eq!(decoded.network_nbt_unchecked(), &expected);
        let refs = decoded.references().collect::<Vec<_>>();
        assert_eq!(refs.len(), 2);
        assert!(refs.iter().all(|reference| {
            reference.target_registry == MOB_EFFECT_REGISTRY
                && reference.kind == RegistryReferenceKind::Element
        }));
    }

    let duplicate = network(&compound(vec![
        entry("speed", compound(vec![])),
        entry("minecraft:speed", compound(vec![])),
    ]));
    assert!(
        decode_mob_effects_predicate(duplicate.root(), "$", &mut context(duplicate.source))
            .is_err()
    );
}

#[test]
fn criterion_entity_leaf_equipment_reuses_item_component_recursion() {
    let data = decode_direct_registry_json(
        br#"{"head":{"items":"diamond_helmet","components":{"custom_name":"hello"}}}"#,
    )
    .unwrap();
    let decoded =
        decode_entity_equipment_predicate(data.root(), "$", &mut context(data.source)).unwrap();
    assert_eq!(decoded.references().count(), 1);
    assert!(matches!(
        decoded.network_nbt_unchecked(),
        NbtSummaryValue::Compound(_)
    ));
}

#[test]
fn criterion_entity_leaf_all_nine_stat_dispatches_and_custom_reference_are_preserved() {
    assert_eq!(VANILLA_STAT_TYPES.len(), 9);
    for stat_type in VANILLA_STAT_TYPES {
        let json = format!(r#"[{{"type":"{}","stat":"test"}}]"#, stat_type.resource_id);
        let data = decode_direct_registry_json(json.as_bytes()).unwrap();
        let decoded = decode_player_stats(data.root(), "$", &mut context(data.source)).unwrap();
        let refs = decoded.references().collect::<Vec<_>>();
        assert_eq!(refs.len(), 1, "{}", stat_type.resource_id);
        assert_eq!(refs[0].target_registry, stat_type.value_registry);
        assert_eq!(refs[0].kind, RegistryReferenceKind::Element);
    }

    let json = br#"[{"type":"custom","stat":"test"}]"#;
    let expected = list(vec![compound(vec![
        entry("stat", string("minecraft:test")),
        entry("type", string("minecraft:custom")),
    ])]);
    for data in [
        decode_direct_registry_json(json).unwrap(),
        decode_client_known_pack_registry_json(json).unwrap(),
        network(&expected),
    ] {
        let decoded = decode_player_stats(data.root(), "$", &mut context(data.source)).unwrap();
        assert_eq!(decoded.network_nbt_unchecked(), &expected);
        let refs = decoded.references().collect::<Vec<_>>();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].target_registry, "minecraft:custom_stat");
        assert_eq!(refs[0].kind, RegistryReferenceKind::Element);
    }
}
