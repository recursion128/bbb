//! Central source-aware dispatcher for synchronized registry elements.

use bbb_facts::registry_data_types::synchronized_registry_data_type;
use bbb_protocol::{content_registry::ContentRegistry, NbtSummaryValue};

use super::{
    animal_variants::{
        animal_variant_registry_codec_references, decode_animal_variant_element,
        encode_animal_variant_network, AnimalVariantElement,
    },
    biome_source::decode_biome_element_with_content_registry,
    chat_type_source::decode_chat_type_element_with_content_registry,
    dialog_source::{decode_dialog_source, DIALOG_CODEC, DIALOG_REGISTRY},
    dimension_type_source::{
        decode_dimension_type_element_with_content_registry, DecodedDimensionType,
    },
    direct_leaf::{
        decode_direct_leaf_registry_element, encode_direct_leaf_registry_element_network,
        DirectLeafRegistryElement,
    },
    enchantment_source::decode_enchantment_element_with_content_registry,
    game_tests::{
        decode_game_test_registry_element, encode_game_test_registry_element_network_unchecked,
        game_test_registry_element_references, GameTestRegistryElement,
    },
    instrument_jukebox_source::{
        decode_instrument_element_with_content_registry,
        decode_jukebox_song_element_with_content_registry,
    },
    painting_variant_source::decode_painting_variant_element_with_content_registry,
    recursive_values::{
        CanonicalRegistryCodecValue, OwnedRegistryCodecReference, RegistryCodecDecodeContext,
    },
    sound_variants::{
        decode_sound_variant_element, encode_sound_variant_network,
        sound_variant_registry_codec_references, SoundVariantElement,
    },
    synchronized_registry_element_codec,
    timeline_source::decode_timeline_element_with_content_registry,
    trim_source::{
        decode_trim_material_element_with_content_registry,
        decode_trim_pattern_element_with_content_registry,
    },
    validate_registry_codec_references, RegistryCodecError, RegistryCodecReference,
    RegistryElementData, RegistryElementSource, RegistryReferenceLookup,
};

#[derive(Debug, Clone, PartialEq)]
pub struct DecodedSynchronizedRegistryElement {
    registry: &'static str,
    value: SynchronizedRegistryElementValue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SynchronizedRegistryElementValue {
    DirectLeaf(DirectLeafRegistryElement),
    AnimalVariant(AnimalVariantElement),
    SoundVariant(SoundVariantElement),
    GameTest(GameTestRegistryElement),
    DimensionType(DecodedDimensionType),
    Canonical(CanonicalRegistryCodecValue),
}

/// Decodes one element through its source-aware synchronized codec.
pub fn decode_exact_synchronized_registry_element(
    registry: &str,
    data: &RegistryElementData,
) -> Result<DecodedSynchronizedRegistryElement, RegistryCodecError> {
    decode_exact_synchronized_registry_element_with_content_registry(
        registry,
        data,
        ContentRegistry::vanilla(),
    )
}

/// Registry-aware form of [`decode_exact_synchronized_registry_element`].
pub fn decode_exact_synchronized_registry_element_with_content_registry(
    registry: &str,
    data: &RegistryElementData,
    content_registry: &ContentRegistry,
) -> Result<DecodedSynchronizedRegistryElement, RegistryCodecError> {
    let data_type = synchronized_registry_data_type(registry).ok_or_else(|| {
        RegistryCodecError::native_semantic(
            registry,
            "RegistryDataLoader.catalog",
            "<registry>",
            "registry is missing from the generated synchronized catalog",
        )
        .aborting_speculation()
    })?;
    let value = match registry {
        "minecraft:damage_type" | "minecraft:banner_pattern" | "minecraft:world_clock" => {
            SynchronizedRegistryElementValue::DirectLeaf(decode_direct_leaf_registry_element(
                registry, data,
            )?)
        }
        "minecraft:wolf_variant"
        | "minecraft:pig_variant"
        | "minecraft:frog_variant"
        | "minecraft:cat_variant"
        | "minecraft:cow_variant"
        | "minecraft:chicken_variant"
        | "minecraft:zombie_nautilus_variant" => SynchronizedRegistryElementValue::AnimalVariant(
            decode_animal_variant_element(registry, data)?,
        ),
        "minecraft:wolf_sound_variant"
        | "minecraft:pig_sound_variant"
        | "minecraft:cat_sound_variant"
        | "minecraft:cow_sound_variant"
        | "minecraft:chicken_sound_variant" => SynchronizedRegistryElementValue::SoundVariant(
            decode_sound_variant_element(registry, data.source, data.root())?,
        ),
        "minecraft:test_environment" | "minecraft:test_instance" => {
            SynchronizedRegistryElementValue::GameTest(decode_game_test_registry_element(
                registry, data,
            )?)
        }
        DIALOG_REGISTRY => {
            let mut context =
                RegistryCodecDecodeContext::new(DIALOG_REGISTRY, DIALOG_CODEC, data.source)
                    .with_content_registry(content_registry);
            SynchronizedRegistryElementValue::Canonical(decode_dialog_source(
                data.root(),
                "<root>",
                &mut context,
            )?)
        }
        "minecraft:dimension_type" => SynchronizedRegistryElementValue::DimensionType(
            decode_dimension_type_element_with_content_registry(data, content_registry)?,
        ),
        "minecraft:enchantment" => SynchronizedRegistryElementValue::Canonical(
            decode_enchantment_element_with_content_registry(data, content_registry)?,
        ),
        "minecraft:timeline" => SynchronizedRegistryElementValue::Canonical(
            decode_timeline_element_with_content_registry(data, content_registry)?,
        ),
        "minecraft:chat_type" => SynchronizedRegistryElementValue::Canonical(
            decode_chat_type_element_with_content_registry(data, content_registry)?,
        ),
        "minecraft:trim_pattern" => SynchronizedRegistryElementValue::Canonical(
            decode_trim_pattern_element_with_content_registry(data, content_registry)?,
        ),
        "minecraft:trim_material" => SynchronizedRegistryElementValue::Canonical(
            decode_trim_material_element_with_content_registry(data, content_registry)?,
        ),
        "minecraft:painting_variant" => SynchronizedRegistryElementValue::Canonical(
            decode_painting_variant_element_with_content_registry(data, content_registry)?,
        ),
        "minecraft:jukebox_song" => SynchronizedRegistryElementValue::Canonical(
            decode_jukebox_song_element_with_content_registry(data, content_registry)?,
        ),
        "minecraft:instrument" => SynchronizedRegistryElementValue::Canonical(
            decode_instrument_element_with_content_registry(data, content_registry)?,
        ),
        "minecraft:worldgen/biome" => SynchronizedRegistryElementValue::Canonical(
            decode_biome_element_with_content_registry(data, content_registry)?,
        ),
        _ => {
            return Err(RegistryCodecError::native_semantic(
                registry,
                "RegistryDataLoader.catalog",
                "<registry>",
                "exact synchronized registry has no dispatcher owner",
            )
            .aborting_speculation())
        }
    };
    Ok(DecodedSynchronizedRegistryElement {
        registry: data_type.resource_id,
        value,
    })
}

impl DecodedSynchronizedRegistryElement {
    pub fn into_canonical(self) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
        match self {
            Self {
                value: SynchronizedRegistryElementValue::Canonical(value),
                ..
            } => Ok(value),
            decoded => {
                let references = decoded
                    .references()
                    .into_iter()
                    .map(|reference| OwnedRegistryCodecReference {
                        target_registry: reference.target_registry,
                        kind: reference.kind,
                        resource_id: reference.resource_id.clone(),
                    })
                    .collect();
                Ok(CanonicalRegistryCodecValue {
                    network_nbt: decoded.projected_network_nbt()?,
                    references,
                })
            }
        }
    }

    pub fn into_value(self) -> SynchronizedRegistryElementValue {
        self.value
    }

    pub fn dimension_type(&self) -> Option<&DecodedDimensionType> {
        match &self.value {
            SynchronizedRegistryElementValue::DimensionType(value) => Some(value),
            _ => None,
        }
    }

    /// Enumerates every named holder/tag placeholder without dereferencing
    /// its target. Order follows the owning element codec.
    pub fn references(&self) -> Vec<RegistryCodecReference<'_>> {
        match &self.value {
            SynchronizedRegistryElementValue::DirectLeaf(_) => Vec::new(),
            SynchronizedRegistryElementValue::AnimalVariant(value) => {
                animal_variant_registry_codec_references(value)
            }
            SynchronizedRegistryElementValue::SoundVariant(value) => {
                sound_variant_registry_codec_references(value)
            }
            SynchronizedRegistryElementValue::GameTest(value) => {
                game_test_registry_element_references(value)
            }
            SynchronizedRegistryElementValue::Canonical(value) => value.references().collect(),
            SynchronizedRegistryElementValue::DimensionType(value) => value
                .references
                .iter()
                .map(|reference| reference.borrowed())
                .collect(),
        }
    }

    /// Network-codec projection of a decoded element, without membership checks.
    ///
    /// Server load retains this NBT beside the winning source bytes. Freeze
    /// then proves every placeholder against the complete generation, and
    /// publication writes the retained tag.
    pub fn projected_network_nbt(self) -> Result<NbtSummaryValue, RegistryCodecError> {
        match self.value {
            SynchronizedRegistryElementValue::DirectLeaf(value) => {
                encode_direct_leaf_registry_element_network(&value)
            }
            SynchronizedRegistryElementValue::AnimalVariant(value) => {
                encode_animal_variant_network(&value)
            }
            SynchronizedRegistryElementValue::SoundVariant(value) => {
                encode_sound_variant_network(&value)
            }
            SynchronizedRegistryElementValue::GameTest(value) => {
                encode_game_test_registry_element_network_unchecked(&value)
            }
            SynchronizedRegistryElementValue::Canonical(value) => Ok(value.network_nbt),
            SynchronizedRegistryElementValue::DimensionType(value) => {
                Ok(value.projected_network_nbt())
            }
        }
    }

    /// Validates every placeholder against one complete frozen generation,
    /// then emits the canonical NBT produced by the synchronized network
    /// codec. No family may encode before this membership check.
    pub fn encode_network(
        self,
        lookup: &impl RegistryReferenceLookup,
    ) -> Result<NbtSummaryValue, RegistryCodecError> {
        self.validate_references(lookup)?;
        self.projected_network_nbt()
    }

    pub fn validate_references(
        &self,
        lookup: &impl RegistryReferenceLookup,
    ) -> Result<(), RegistryCodecError> {
        let codec =
            synchronized_registry_element_codec(self.registry, RegistryElementSource::NetworkNbt)?;
        validate_registry_codec_references(self.registry, codec, self.references(), lookup)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use crate::{decode_client_known_pack_registry_json, decode_direct_registry_json};
    use bbb_facts::registry_data_types::VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES;

    use super::*;

    fn nbt_field<'a>(value: &'a NbtSummaryValue, name: &str) -> &'a NbtSummaryValue {
        let NbtSummaryValue::Compound(entries) = value else {
            panic!("expected compound field {name}");
        };
        &entries
            .iter()
            .find(|entry| entry.name == name)
            .unwrap()
            .value
    }

    fn nbt_field_mut<'a>(value: &'a mut NbtSummaryValue, name: &str) -> &'a mut NbtSummaryValue {
        let NbtSummaryValue::Compound(entries) = value else {
            panic!("expected compound field {name}");
        };
        &mut entries
            .iter_mut()
            .find(|entry| entry.name == name)
            .unwrap()
            .value
    }

    fn first_effect(value: &NbtSummaryValue) -> &NbtSummaryValue {
        let NbtSummaryValue::List(effects) = value else {
            panic!("expected effect list");
        };
        effects.first().unwrap()
    }

    #[test]
    fn dialog_network_projection_strips_only_potion_hidden_effects() {
        let raw = br#"{"type":"notice","title":"Potion","body":{"type":"item","item":{"id":"potion","components":{"potion_contents":{"custom_name":"\ud800","custom_effects":[{"id":"haste","amplifier":1,"duration":200,"hidden_effect":{"duration":40}}]},"consumable":{"on_consume_effects":[{"type":"apply_effects","effects":[{"id":"haste","hidden_effect":{"duration":40}}]}]}}}}}"#;
        let data = decode_direct_registry_json(raw).unwrap();
        let decoded =
            decode_exact_synchronized_registry_element("minecraft:dialog", &data).unwrap();
        let network = decoded.encode_network(&Lookup::default()).unwrap();
        let components = nbt_field(nbt_field(nbt_field(&network, "body"), "item"), "components");
        let potion = nbt_field(components, "minecraft:potion_contents");
        assert_eq!(
            nbt_field(potion, "custom_name"),
            &NbtSummaryValue::string_from_utf16_slice(&[0xd800]),
        );
        let potion_effect = first_effect(nbt_field(potion, "custom_effects"));
        assert!(
            matches!(potion_effect, NbtSummaryValue::Compound(entries) if !entries.iter().any(|entry| entry.name == "hidden_effect"))
        );
        let generic = nbt_field(components, "minecraft:consumable");
        let generic_effect = first_effect(nbt_field(generic, "on_consume_effects"));
        let generic_effect = first_effect(nbt_field(generic_effect, "effects"));
        assert_eq!(
            nbt_field(generic_effect, "hidden_effect"),
            &NbtSummaryValue::Compound(vec![
                bbb_protocol::NbtSummaryEntry {
                    name: "duration".into(),
                    value: NbtSummaryValue::Int(40)
                },
                bbb_protocol::NbtSummaryEntry {
                    name: "show_icon".into(),
                    value: NbtSummaryValue::Byte(1)
                },
            ])
        );

        let mut supplied_nbt = network.clone();
        let components = nbt_field_mut(
            nbt_field_mut(nbt_field_mut(&mut supplied_nbt, "body"), "item"),
            "components",
        );
        let potion = nbt_field_mut(components, "minecraft:potion_contents");
        let NbtSummaryValue::List(effects) = nbt_field_mut(potion, "custom_effects") else {
            panic!("potion effects");
        };
        let NbtSummaryValue::Compound(effect) = &mut effects[0] else {
            panic!("potion effect");
        };
        effect.push(bbb_protocol::NbtSummaryEntry {
            name: "hidden_effect".into(),
            value: NbtSummaryValue::Compound(vec![bbb_protocol::NbtSummaryEntry {
                name: "duration".into(),
                value: NbtSummaryValue::Int(40),
            }]),
        });
        let nbt_data = RegistryElementData::from_network_nbt(supplied_nbt.clone());
        assert_eq!(
            decode_exact_synchronized_registry_element("minecraft:dialog", &nbt_data)
                .unwrap()
                .projected_network_nbt()
                .unwrap(),
            network,
        );

        let mut invalid = supplied_nbt;
        let components = nbt_field_mut(
            nbt_field_mut(nbt_field_mut(&mut invalid, "body"), "item"),
            "components",
        );
        let potion = nbt_field_mut(components, "minecraft:potion_contents");
        let NbtSummaryValue::List(effects) = nbt_field_mut(potion, "custom_effects") else {
            panic!("potion effects");
        };
        let NbtSummaryValue::Compound(effect) = &mut effects[0] else {
            panic!("potion effect");
        };
        effect
            .iter_mut()
            .find(|entry| entry.name == "hidden_effect")
            .unwrap()
            .value = NbtSummaryValue::Int(1);
        let invalid_data = RegistryElementData::from_network_nbt(invalid);
        assert!(
            decode_exact_synchronized_registry_element("minecraft:dialog", &invalid_data).is_err()
        );
    }

    #[derive(Default)]
    struct Lookup {
        missing: Option<(&'static str, &'static str)>,
    }

    impl RegistryReferenceLookup for Lookup {
        fn contains_element(&self, registry: &str, resource_id: &str) -> bool {
            self.missing != Some((registry, resource_id))
        }

        fn contains_tag(&self, registry: &str, resource_id: &str) -> bool {
            self.missing != Some((registry, resource_id))
        }
    }

    #[test]
    fn every_exact_registry_round_trips_through_the_central_network_encoder() {
        let cases = exact_known_pack_cases();
        assert_eq!(
            cases
                .iter()
                .map(|(registry, _)| *registry)
                .collect::<BTreeSet<_>>(),
            VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES
                .iter()
                .map(|entry| entry.resource_id)
                .collect()
        );
        for (registry, json) in cases {
            let source = decode_client_known_pack_registry_json(json.as_bytes()).unwrap();
            let decoded = decode_exact_synchronized_registry_element(registry, &source)
                .unwrap_or_else(|error| panic!("{registry} known-pack decode failed: {error}"));
            let encoded = decoded
                .encode_network(&Lookup::default())
                .unwrap_or_else(|error| panic!("{registry} network encode failed: {error}"));
            let network = RegistryElementData::from_network_nbt(encoded.clone());
            let round_trip = decode_exact_synchronized_registry_element(registry, &network)
                .unwrap_or_else(|error| panic!("{registry} network decode failed: {error}"));
            assert_eq!(
                round_trip.encode_network(&Lookup::default()).unwrap(),
                encoded,
                "{registry}"
            );
        }
    }

    #[test]
    fn publication_decodes_inline_and_named_biome_generation_as_an_exact_owner() {
        let data = decode_direct_registry_json(
            br##"{
                "has_precipitation":true,
                "temperature":0.8,
                "downfall":0.4,
                "effects":{"water_color":"#3f76e4"},
                "carvers":["minecraft:cave"],
                "features":[[],["minecraft:lake_lava_surface"]],
                "spawners":{},
                "spawn_costs":{}
            }"##,
        )
        .unwrap();
        let decoded =
            decode_exact_synchronized_registry_element("minecraft:worldgen/biome", &data).unwrap();
        assert_eq!(decoded.references().len(), 2);
    }

    #[test]
    fn dimension_tag_key_and_holder_set_tag_reference_validation() {
        let base = r##"{
            "has_skylight":true,
            "has_ceiling":false,
            "has_ender_dragon_fight":false,
            "coordinate_scale":1.0,
            "min_y":0,
            "height":16,
            "logical_height":16,
            "infiniburn":"#example:absent",
            "ambient_light":0.0,
            "monster_spawn_light_level":0,
            "monster_spawn_block_light_limit":0
        }"##;
        let data = decode_direct_registry_json(base.as_bytes()).unwrap();
        struct UnavailableLookup;
        impl RegistryReferenceLookup for UnavailableLookup {
            fn contains_element(&self, _: &str, _: &str) -> bool {
                false
            }
            fn contains_tag(&self, _: &str, _: &str) -> bool {
                false
            }
            fn element_membership(
                &self,
                _: &str,
                _: &str,
            ) -> crate::registry_codec::RegistryReferenceMembership {
                crate::registry_codec::RegistryReferenceMembership::Unavailable
            }
            fn tag_membership(
                &self,
                _: &str,
                _: &str,
            ) -> crate::registry_codec::RegistryReferenceMembership {
                crate::registry_codec::RegistryReferenceMembership::Unavailable
            }
        }

        let decoded =
            decode_exact_synchronized_registry_element("minecraft:dimension_type", &data).unwrap();
        decoded
            .encode_network(&Lookup {
                missing: Some(("minecraft:block", "example:absent")),
            })
            .unwrap();

        let with_timeline = base.replacen(
            "\n        }",
            ",\n            \"timelines\":\"#example:missing\"\n        }",
            1,
        );
        let data = decode_direct_registry_json(with_timeline.as_bytes()).unwrap();
        let err = decode_exact_synchronized_registry_element("minecraft:dimension_type", &data)
            .unwrap()
            .encode_network(&Lookup {
                missing: Some(("minecraft:timeline", "example:missing")),
            })
            .unwrap_err();
        assert!(
            err.to_string()
                .contains("missing tag example:missing in minecraft:timeline"),
            "{err}"
        );

        let decoded =
            decode_exact_synchronized_registry_element("minecraft:dimension_type", &data).unwrap();
        let unavailable_err = validate_registry_codec_references(
            "minecraft:dimension_type",
            "DimensionType.NETWORK_CODEC",
            decoded.references(),
            &UnavailableLookup,
        )
        .unwrap_err();
        assert!(
            unavailable_err
                .to_string()
                .contains("registry minecraft:timeline is unavailable"),
            "{unavailable_err}"
        );

        let network = decoded.encode_network(&Lookup::default()).unwrap();
        assert_eq!(
            nbt_field(&network, "timelines"),
            &NbtSummaryValue::String("#example:missing".into())
        );
    }

    fn exact_known_pack_cases() -> Vec<(&'static str, &'static str)> {
        let wolf_sounds = r#"{
            "adult_sounds":{
                "ambient_sound":"minecraft:entity.wolf.ambient","death_sound":"minecraft:entity.wolf.death",
                "growl_sound":"minecraft:entity.wolf.growl","hurt_sound":"minecraft:entity.wolf.hurt",
                "pant_sound":"minecraft:entity.wolf.pant","whine_sound":"minecraft:entity.wolf.whine",
                "step_sound":"minecraft:entity.wolf.step"
            },
            "baby_sounds":{
                "ambient_sound":"minecraft:entity.wolf.ambient","death_sound":"minecraft:entity.wolf.death",
                "growl_sound":"minecraft:entity.wolf.growl","hurt_sound":"minecraft:entity.wolf.hurt",
                "pant_sound":"minecraft:entity.wolf.pant","whine_sound":"minecraft:entity.wolf.whine",
                "step_sound":"minecraft:entity.wolf.step"
            }
        }"#;
        let pig_sounds = r#"{
            "adult_sounds":{
                "ambient_sound":"minecraft:entity.pig.ambient","hurt_sound":"minecraft:entity.pig.hurt",
                "death_sound":"minecraft:entity.pig.death","step_sound":"minecraft:entity.pig.step",
                "eat_sound":"minecraft:entity.pig.eat"
            },
            "baby_sounds":{
                "ambient_sound":"minecraft:entity.pig.ambient","hurt_sound":"minecraft:entity.pig.hurt",
                "death_sound":"minecraft:entity.pig.death","step_sound":"minecraft:entity.pig.step",
                "eat_sound":"minecraft:entity.pig.eat"
            }
        }"#;
        let cat_sounds = r#"{
            "adult_sounds":{
                "ambient_sound":"minecraft:entity.cat.ambient","stray_ambient_sound":"minecraft:entity.cat.stray_ambient",
                "hiss_sound":"minecraft:entity.cat.hiss","hurt_sound":"minecraft:entity.cat.hurt",
                "death_sound":"minecraft:entity.cat.death","eat_sound":"minecraft:entity.cat.eat",
                "beg_for_food_sound":"minecraft:entity.cat.beg_for_food","purr_sound":"minecraft:entity.cat.purr",
                "purreow_sound":"minecraft:entity.cat.purreow"
            },
            "baby_sounds":{
                "ambient_sound":"minecraft:entity.cat.ambient","stray_ambient_sound":"minecraft:entity.cat.stray_ambient",
                "hiss_sound":"minecraft:entity.cat.hiss","hurt_sound":"minecraft:entity.cat.hurt",
                "death_sound":"minecraft:entity.cat.death","eat_sound":"minecraft:entity.cat.eat",
                "beg_for_food_sound":"minecraft:entity.cat.beg_for_food","purr_sound":"minecraft:entity.cat.purr",
                "purreow_sound":"minecraft:entity.cat.purreow"
            }
        }"#;
        let chicken_sounds = r#"{
            "adult_sounds":{
                "ambient_sound":"minecraft:entity.chicken.ambient","hurt_sound":"minecraft:entity.chicken.hurt",
                "death_sound":"minecraft:entity.chicken.death","step_sound":"minecraft:entity.chicken.step"
            },
            "baby_sounds":{
                "ambient_sound":"minecraft:entity.chicken.ambient","hurt_sound":"minecraft:entity.chicken.hurt",
                "death_sound":"minecraft:entity.chicken.death","step_sound":"minecraft:entity.chicken.step"
            }
        }"#;
        vec![
            (
                "minecraft:wolf_variant",
                r#"{"assets":{"wild":"a","tame":"b","angry":"c"},"baby_assets":{"wild":"d","tame":"e","angry":"f"}}"#,
            ),
            ("minecraft:wolf_sound_variant", wolf_sounds),
            (
                "minecraft:pig_variant",
                r#"{"asset_id":"a","baby_asset_id":"b"}"#,
            ),
            ("minecraft:pig_sound_variant", pig_sounds),
            ("minecraft:frog_variant", r#"{"asset_id":"a"}"#),
            (
                "minecraft:cat_variant",
                r#"{"asset_id":"a","baby_asset_id":"b"}"#,
            ),
            ("minecraft:cat_sound_variant", cat_sounds),
            (
                "minecraft:cow_sound_variant",
                r#"{"ambient_sound":"minecraft:entity.cow.ambient","hurt_sound":"minecraft:entity.cow.hurt","death_sound":"minecraft:entity.cow.death","step_sound":"minecraft:entity.cow.step"}"#,
            ),
            (
                "minecraft:cow_variant",
                r#"{"asset_id":"a","baby_asset_id":"b"}"#,
            ),
            ("minecraft:chicken_sound_variant", chicken_sounds),
            (
                "minecraft:chicken_variant",
                r#"{"asset_id":"a","baby_asset_id":"b"}"#,
            ),
            ("minecraft:zombie_nautilus_variant", r#"{"asset_id":"a"}"#),
            (
                "minecraft:damage_type",
                r#"{"message_id":"generic","scaling":"never","exhaustion":0}"#,
            ),
            (
                "minecraft:banner_pattern",
                r#"{"asset_id":"base","translation_key":"block.minecraft.banner.base"}"#,
            ),
            (
                "minecraft:test_environment",
                r#"{"type":"weather","weather":"clear"}"#,
            ),
            (
                "minecraft:test_instance",
                r#"{"type":"block_based","environment":{"type":"weather","weather":"clear"},"structure":"example:room","max_ticks":20}"#,
            ),
            ("minecraft:world_clock", r#"{}"#),
            (
                "minecraft:dimension_type",
                r##"{"has_skylight":true,"has_ceiling":false,"has_ender_dragon_fight":false,"coordinate_scale":1.0,"min_y":0,"height":16,"logical_height":16,"infiniburn":"#minecraft:infiniburn_overworld","ambient_light":0.0,"monster_spawn_light_level":0,"monster_spawn_block_light_limit":0}"##,
            ),
            (
                "minecraft:enchantment",
                r#"{"description":"Test","supported_items":"minecraft:stone","weight":1,"max_level":1,"min_cost":{"base":1,"per_level_above_first":0},"max_cost":{"base":1,"per_level_above_first":0},"anvil_cost":0,"slots":["mainhand"]}"#,
            ),
            ("minecraft:timeline", r#"{"clock":"example:clock"}"#),
            (
                "minecraft:chat_type",
                r#"{"chat":{"translation_key":"chat.type.text","parameters":["sender","content"]},"narration":{"translation_key":"chat.type.text.narrate","parameters":["sender","content"]}}"#,
            ),
            (
                "minecraft:trim_pattern",
                r#"{"asset_id":"coast","description":"Coast"}"#,
            ),
            (
                "minecraft:trim_material",
                r#"{"asset_name":"gold","description":"Gold"}"#,
            ),
            (
                "minecraft:painting_variant",
                r#"{"width":1,"height":1,"asset_id":"kebab"}"#,
            ),
            (
                "minecraft:jukebox_song",
                r#"{"sound_event":"minecraft:ambient.cave","description":"Song","length_in_seconds":1.0,"comparator_output":0}"#,
            ),
            (
                "minecraft:instrument",
                r#"{"sound_event":"minecraft:ambient.cave","use_duration":1.0,"range":16.0,"description":"Horn"}"#,
            ),
            (
                "minecraft:dialog",
                r#"{"type":"notice","title":{"translate":"gui.ok"}}"#,
            ),
            (
                "minecraft:worldgen/biome",
                r##"{"has_precipitation":true,"temperature":0.8,"downfall":0.4,"effects":{"water_color":"#3f76e4"}}"##,
            ),
        ]
    }
}
