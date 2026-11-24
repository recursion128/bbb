use crate::{
    decode_client_known_pack_registry_json, decode_direct_registry_json,
    decode_network_registry_element,
    enchantment_codec_types::{
        VANILLA_ENCHANTMENT_ENTITY_EFFECT_CODEC_TYPES,
        VANILLA_ENCHANTMENT_LOCATION_BASED_EFFECT_CODEC_TYPES,
    },
    registry_codec::{
        recursive_values::RegistryCodecDecodeContext, RegistryElementData, RegistryElementSource,
    },
};
use bbb_protocol::{
    encode_nbt_summary,
    nbt_summary::{canonical_double_tag_bits, canonical_float_tag_bits},
    NbtSummaryEntry,
};

use super::*;
use bbb_protocol::nbt_summary::entry;
use bbb_protocol::NbtSummaryValue;

fn context(source: RegistryElementSource) -> RegistryCodecDecodeContext<'static> {
    RegistryCodecDecodeContext::new("minecraft:enchantment", "Enchantment.DIRECT_CODEC", source)
}

fn network(value: &NbtSummaryValue) -> RegistryElementData {
    decode_network_registry_element(&encode_nbt_summary(value, 1 << 20).unwrap()).unwrap()
}

fn compound(fields: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(fields)
}

fn float(value: f32) -> NbtSummaryValue {
    NbtSummaryValue::Float(canonical_float_tag_bits(value))
}

fn double(value: f64) -> NbtSummaryValue {
    NbtSummaryValue::Double(canonical_double_tag_bits(value))
}

fn effect(name: &str, fields: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    let mut fields = fields;
    fields.push(entry(
        "type",
        NbtSummaryValue::String(format!("minecraft:{name}").into()),
    ));
    compound(fields)
}

fn decode_entity(data: &RegistryElementData) -> CanonicalRegistryCodecValue {
    decode_enchantment_entity_effect_source(data.root(), "$", &mut context(data.source)).unwrap()
}

fn decode_location(data: &RegistryElementData) -> CanonicalRegistryCodecValue {
    decode_enchantment_location_based_effect_source(data.root(), "$", &mut context(data.source))
        .unwrap()
}

#[test]
fn ten_leaf_families_write_exact_trees_for_all_three_sources() {
    let cases = [
        (
            false,
            r#"{"type":"apply_mob_effect","to_apply":"speed","min_duration":1,"max_duration":2,"min_amplifier":0,"max_amplifier":1}"#,
            effect(
                "apply_mob_effect",
                vec![
                    entry(
                        "to_apply",
                        NbtSummaryValue::String("minecraft:speed".to_string().into()),
                    ),
                    entry("min_duration", float(1.0)),
                    entry("max_duration", float(2.0)),
                    entry("min_amplifier", float(0.0)),
                    entry("max_amplifier", float(1.0)),
                ],
            ),
        ),
        (
            false,
            r#"{"type":"change_item_damage","amount":1}"#,
            effect("change_item_damage", vec![entry("amount", float(1.0))]),
        ),
        (
            false,
            r#"{"type":"damage_entity","min_damage":1,"max_damage":2,"damage_type":"generic"}"#,
            effect(
                "damage_entity",
                vec![
                    entry("min_damage", float(1.0)),
                    entry("max_damage", float(2.0)),
                    entry(
                        "damage_type",
                        NbtSummaryValue::String("minecraft:generic".to_string().into()),
                    ),
                ],
            ),
        ),
        (
            false,
            r#"{"type":"ignite","duration":3}"#,
            effect("ignite", vec![entry("duration", float(3.0))]),
        ),
        (
            false,
            r#"{"type":"apply_impulse","direction":[1,2,3],"coordinate_scale":[4,5,6],"magnitude":2}"#,
            effect(
                "apply_impulse",
                vec![
                    entry(
                        "direction",
                        NbtSummaryValue::List(vec![double(1.0), double(2.0), double(3.0)]),
                    ),
                    entry(
                        "coordinate_scale",
                        NbtSummaryValue::List(vec![double(4.0), double(5.0), double(6.0)]),
                    ),
                    entry("magnitude", float(2.0)),
                ],
            ),
        ),
        (
            false,
            r#"{"type":"apply_exhaustion","amount":0.5}"#,
            effect("apply_exhaustion", vec![entry("amount", float(0.5))]),
        ),
        (
            false,
            r#"{"type":"play_sound","sound":"entity.player.levelup","volume":1,"pitch":1}"#,
            effect(
                "play_sound",
                vec![
                    entry(
                        "sound",
                        NbtSummaryValue::String(
                            "minecraft:entity.player.levelup".to_string().into(),
                        ),
                    ),
                    entry("volume", float(1.0)),
                    entry("pitch", float(1.0)),
                ],
            ),
        ),
        (
            false,
            r#"{"type":"run_function","function":"example:test"}"#,
            effect(
                "run_function",
                vec![entry(
                    "function",
                    NbtSummaryValue::String("example:test".to_string().into()),
                )],
            ),
        ),
        (
            false,
            r#"{"type":"summon_entity","entity":"pig","join_team":true}"#,
            effect(
                "summon_entity",
                vec![
                    entry(
                        "entity",
                        NbtSummaryValue::String("minecraft:pig".to_string().into()),
                    ),
                    entry("join_team", NbtSummaryValue::Byte(1)),
                ],
            ),
        ),
        (
            true,
            r#"{"type":"attribute","id":"example:power","attribute":"generic.attack_damage","amount":1,"operation":"add_value"}"#,
            effect(
                "attribute",
                vec![
                    entry(
                        "id",
                        NbtSummaryValue::String("example:power".to_string().into()),
                    ),
                    entry(
                        "attribute",
                        NbtSummaryValue::String(
                            "minecraft:generic.attack_damage".to_string().into(),
                        ),
                    ),
                    entry("amount", float(1.0)),
                    entry(
                        "operation",
                        NbtSummaryValue::String("add_value".to_string().into()),
                    ),
                ],
            ),
        ),
    ];

    for (location_only, json, expected) in cases {
        for data in [
            decode_direct_registry_json(json.as_bytes()).unwrap(),
            decode_client_known_pack_registry_json(json.as_bytes()).unwrap(),
            network(&expected),
        ] {
            let decoded = if location_only {
                decode_location(&data)
            } else {
                decode_entity(&data)
            };
            assert_eq!(decoded.network_nbt_unchecked(), &expected, "{json}");
            if !location_only {
                assert_eq!(
                    decode_location(&data).network_nbt_unchecked(),
                    &expected,
                    "shared location family: {json}"
                );
            }
        }
    }
}

#[test]
fn generated_catalogs_are_exhaustive_and_all_of_recurses_against_its_own_catalog() {
    assert_eq!(VANILLA_ENCHANTMENT_ENTITY_EFFECT_CODEC_TYPES.len(), 15);
    assert_eq!(
        VANILLA_ENCHANTMENT_LOCATION_BASED_EFFECT_CODEC_TYPES.len(),
        16
    );

    let location = decode_direct_registry_json(
        br#"{"type":"all_of","effects":[{"type":"attribute","id":"example:power","attribute":"generic.attack_damage","amount":1,"operation":"add_value"}]}"#,
    )
    .unwrap();
    let decoded = decode_location(&location);
    assert!(matches!(
        decoded.network_nbt_unchecked(),
        NbtSummaryValue::Compound(fields)
            if fields.first().is_some_and(|field| field.name == "effects")
                && fields.last().is_some_and(|field| field.name == "type")
    ));
    assert!(decode_enchantment_entity_effect_source(
        location.root(),
        "$",
        &mut context(location.source),
    )
    .is_err());

    let empty = decode_direct_registry_json(br#"{"type":"all_of","effects":[]}"#).unwrap();
    assert_eq!(
        decode_entity(&empty).network_nbt_unchecked(),
        &effect(
            "all_of",
            vec![entry("effects", NbtSummaryValue::List(vec![]))]
        )
    );
}

#[test]
fn compact_sound_accepts_empty_and_references_preserve_codec_order() {
    let data =
        decode_direct_registry_json(br#"{"type":"play_sound","sound":[],"volume":1,"pitch":1}"#)
            .unwrap();
    let decoded = decode_entity(&data);
    assert_eq!(decoded.references().count(), 0);
    assert!(matches!(
        decoded.network_nbt_unchecked(),
        NbtSummaryValue::Compound(fields)
            if fields.first().is_some_and(|field| {
                field.name == "sound" && field.value == NbtSummaryValue::List(vec![])
            })
    ));

    let data = decode_direct_registry_json(
        br#"{"type":"play_sound","sound":["entity.player.levelup","entity.generic.explode"],"volume":1,"pitch":1}"#,
    )
    .unwrap();
    let decoded = decode_entity(&data);
    let references: Vec<_> = decoded
        .references()
        .map(|reference| reference.resource_id.as_str().to_string())
        .collect();
    assert_eq!(
        references,
        [
            "minecraft:entity.player.levelup",
            "minecraft:entity.generic.explode"
        ]
    );

    let data = decode_direct_registry_json(
        br#"{"type":"play_sound","sound":["entity.player.levelup"],"volume":1,"pitch":1}"#,
    )
    .unwrap();
    let decoded = decode_entity(&data);
    assert!(matches!(
        decoded.network_nbt_unchecked(),
        NbtSummaryValue::Compound(fields)
            if fields.first().is_some_and(|field| {
                field.name == "sound"
                    && field.value
                        == NbtSummaryValue::String(
                            "minecraft:entity.player.levelup".to_string().into()
                        )
            })
    ));
}
