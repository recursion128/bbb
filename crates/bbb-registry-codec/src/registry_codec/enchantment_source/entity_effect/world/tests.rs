use crate::{
    decode_client_known_pack_registry_json, decode_direct_registry_json,
    decode_network_registry_element,
    registry_codec::{
        recursive_values::{CanonicalRegistryCodecValue, RegistryCodecDecodeContext},
        RegistryElementData, RegistryElementSource, RegistryReferenceKind,
    },
};
use bbb_protocol::encode_nbt_summary;

use super::*;
use bbb_protocol::nbt_summary::entry;

fn context(source: RegistryElementSource) -> RegistryCodecDecodeContext<'static> {
    RegistryCodecDecodeContext::new(
        "minecraft:enchantment",
        "EnchantmentEntityEffect.CODEC",
        source,
    )
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

fn particle(name: &str) -> NbtSummaryValue {
    compound(vec![entry(
        "type",
        NbtSummaryValue::String(format!("minecraft:{name}").into()),
    )])
}

fn provider(name: &str, fields: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    let mut fields = fields;
    fields.push(entry(
        "type",
        NbtSummaryValue::String(format!("minecraft:{name}").into()),
    ));
    compound(fields)
}

fn decode(
    family: EnchantmentEntityEffectCodecFamily,
    data: &RegistryElementData,
) -> CanonicalRegistryCodecValue {
    decode_entity_world_effect_source(
        family,
        match family {
            EnchantmentEntityEffectCodecFamily::ExplodeEffect => "minecraft:explode",
            EnchantmentEntityEffectCodecFamily::ReplaceBlock => "minecraft:replace_block",
            EnchantmentEntityEffectCodecFamily::ReplaceDisk => "minecraft:replace_disk",
            EnchantmentEntityEffectCodecFamily::SetBlockProperties => {
                "minecraft:set_block_properties"
            }
            EnchantmentEntityEffectCodecFamily::SpawnParticlesEffect => "minecraft:spawn_particles",
            _ => panic!("not a world family"),
        }
        .to_string(),
        data.root(),
        "$",
        &mut context(data.source),
    )
    .expect("world family")
    .unwrap()
}

#[test]
fn all_five_world_families_write_exact_trees_for_three_sources() {
    let cases = [
        (
            EnchantmentEntityEffectCodecFamily::ExplodeEffect,
            r#"{"type":"explode","radius":2,"block_interaction":"none","small_particle":{"type":"poof"},"large_particle":{"type":"explosion"},"sound":"entity.generic.explode"}"#,
            provider(
                "explode",
                vec![
                    entry("radius", float(2.0)),
                    entry(
                        "block_interaction",
                        NbtSummaryValue::String("none".to_string().into()),
                    ),
                    entry("small_particle", particle("poof")),
                    entry("large_particle", particle("explosion")),
                    entry(
                        "sound",
                        NbtSummaryValue::String(
                            "minecraft:entity.generic.explode".to_string().into(),
                        ),
                    ),
                ],
            ),
        ),
        (
            EnchantmentEntityEffectCodecFamily::ReplaceBlock,
            r#"{"type":"replace_block","block_state":{"type":"simple_state_provider","state":{"Name":"stone"}}}"#,
            provider(
                "replace_block",
                vec![entry(
                    "block_state",
                    provider(
                        "simple_state_provider",
                        vec![entry(
                            "state",
                            compound(vec![entry(
                                "Name",
                                NbtSummaryValue::String("minecraft:stone".to_string().into()),
                            )]),
                        )],
                    ),
                )],
            ),
        ),
        (
            EnchantmentEntityEffectCodecFamily::ReplaceDisk,
            r#"{"type":"replace_disk","radius":2,"height":1,"block_state":{"type":"simple_state_provider","state":{"Name":"stone"}}}"#,
            provider(
                "replace_disk",
                vec![
                    entry("radius", float(2.0)),
                    entry("height", float(1.0)),
                    entry(
                        "block_state",
                        provider(
                            "simple_state_provider",
                            vec![entry(
                                "state",
                                compound(vec![entry(
                                    "Name",
                                    NbtSummaryValue::String("minecraft:stone".to_string().into()),
                                )]),
                            )],
                        ),
                    ),
                ],
            ),
        ),
        (
            EnchantmentEntityEffectCodecFamily::SetBlockProperties,
            r#"{"type":"set_block_properties","properties":{"facing":"north"}}"#,
            provider(
                "set_block_properties",
                vec![entry(
                    "properties",
                    compound(vec![entry(
                        "facing",
                        NbtSummaryValue::String("north".to_string().into()),
                    )]),
                )],
            ),
        ),
        (
            EnchantmentEntityEffectCodecFamily::SpawnParticlesEffect,
            r#"{"type":"spawn_particles","particle":{"type":"poof"},"horizontal_position":{"type":"entity_position"},"vertical_position":{"type":"in_bounding_box"},"horizontal_velocity":{},"vertical_velocity":{}}"#,
            provider(
                "spawn_particles",
                vec![
                    entry("particle", particle("poof")),
                    entry(
                        "horizontal_position",
                        compound(vec![entry(
                            "type",
                            NbtSummaryValue::String("entity_position".to_string().into()),
                        )]),
                    ),
                    entry(
                        "vertical_position",
                        compound(vec![entry(
                            "type",
                            NbtSummaryValue::String("in_bounding_box".to_string().into()),
                        )]),
                    ),
                    entry("horizontal_velocity", compound(vec![])),
                    entry("vertical_velocity", compound(vec![])),
                ],
            ),
        ),
    ];

    for (family, json, expected) in cases {
        for data in [
            decode_direct_registry_json(json.as_bytes()).unwrap(),
            decode_client_known_pack_registry_json(json.as_bytes()).unwrap(),
            network(&expected),
        ] {
            assert_eq!(decode(family, &data).network_nbt_unchecked(), &expected);
        }
    }
}

#[test]
fn defaults_weighted_entries_and_references_follow_vanilla_owners() {
    let json = decode_direct_registry_json(
        br##"{
          "type":"explode",
          "damage_type":"generic",
          "immune_blocks":"#wool",
          "offset":[0,-0.0,0],
          "radius":1,
          "block_interaction":"block",
          "small_particle":{"type":"poof"},
          "large_particle":{"type":"explosion"},
          "block_particles":[
            {"particle":{"type":"smoke"},"weight":0},
            {"particle":{"type":"cloud"},"scaling":2,"speed":3,"weight":0}
          ],
          "sound":{"sound_id":"entity.generic.explode","range":8}
        }"##,
    )
    .unwrap();
    let decoded = decode(EnchantmentEntityEffectCodecFamily::ExplodeEffect, &json);
    let names: Vec<_> = match decoded.network_nbt_unchecked() {
        NbtSummaryValue::Compound(fields) => fields.iter().map(|entry| &entry.name).collect(),
        _ => panic!("expected compound"),
    };
    assert_eq!(
        names,
        [
            "damage_type",
            "immune_blocks",
            "offset",
            "radius",
            "block_interaction",
            "small_particle",
            "large_particle",
            "block_particles",
            "sound",
            "type",
        ]
    );
    let NbtSummaryValue::Compound(fields) = decoded.network_nbt_unchecked() else {
        unreachable!("field-name assertion already proved a compound")
    };
    assert_eq!(
        fields
            .iter()
            .find(|entry| entry.name == "offset")
            .map(|entry| &entry.value),
        Some(&NbtSummaryValue::List(vec![
            NbtSummaryValue::Double(0.0_f64.to_bits()),
            NbtSummaryValue::Double(0.0_f64.to_bits()),
            NbtSummaryValue::Double(0.0_f64.to_bits()),
        ]))
    );
    let references: Vec<_> = decoded
        .references()
        .map(|reference| {
            (
                reference.target_registry,
                reference.kind,
                reference.resource_id.as_str().to_string(),
            )
        })
        .collect();
    assert_eq!(references.len(), 2);
    assert_eq!(references[0].0, DAMAGE_TYPE_REGISTRY);
    assert_eq!(references[0].1, RegistryReferenceKind::Element);
    assert_eq!(references[0].2, "minecraft:generic");
    assert_eq!(references[1].0, BLOCK_REGISTRY);
    assert_eq!(references[1].1, RegistryReferenceKind::Tag);
    assert_eq!(references[1].2, "minecraft:wool");
}
