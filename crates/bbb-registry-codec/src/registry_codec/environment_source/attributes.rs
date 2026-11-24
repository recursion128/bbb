//! Exact `EnvironmentAttributeMap` dispatched-map owner.

use std::collections::HashSet;

use crate::registry_codec::{
    recursive_values::{required_field, CanonicalRegistryCodecValue, RegistryCodecDecodeContext},
    RegistryCodecError, RegistryElementValueRef, RegistryResourceIdentifier,
};
use bbb_facts::environment_attribute_types::vanilla_environment_attribute;
use bbb_protocol::{nbt_summary::entry, NbtSummaryValue};

use super::values::{decode_attribute_value_source, decode_modifier_argument_source};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EnvironmentAttributeMapMode {
    /// Persistent `EnvironmentAttributeMap.CODEC`.
    Direct,
    /// Persistent biome climate attributes validated by `CODEC_ONLY_POSITIONAL`.
    PositionalDirect,
    /// Synchronized `NETWORK_CODEC`, which decodes everything before filtering.
    Network,
}

pub(crate) fn decode_environment_attribute_map_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    mode: EnvironmentAttributeMapMode,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let len = context.charge_record(input, field, "environment attributes")?;

    let visible = input.visible_map_entries(len);

    let mut identities = HashSet::with_capacity(visible.len());
    let mut output = Vec::with_capacity(visible.len());
    let mut references = Vec::new();

    for (raw_id, value) in visible {
        let resource_id = RegistryResourceIdentifier::parse_java_string(raw_id)
            .ok_or_else(|| context.codec_error(field, "invalid environment attribute id"))?;
        let descriptor = vanilla_environment_attribute(resource_id.as_str()).ok_or_else(|| {
            context.codec_error(
                field,
                format!("unknown environment attribute {}", resource_id.as_str()),
            )
        })?;
        if !identities.insert(descriptor.resource_id) {
            return Err(context.codec_error(
                field,
                format!("duplicate environment attribute {}", descriptor.resource_id),
            ));
        }
        if mode == EnvironmentAttributeMapMode::PositionalDirect && !descriptor.positional {
            return Err(context.codec_error(
                field,
                format!(
                    "non-positional environment attributes are not allowed here: {}",
                    descriptor.resource_id
                ),
            ));
        }
        let decoded = context.with_nested(field, |context| {
            decode_entry_source(descriptor, value, field, context)
        })?;
        let (value, value_references) = decoded.into_parts();
        references.extend(value_references);

        if mode != EnvironmentAttributeMapMode::Network || descriptor.syncable {
            output.push(entry(resource_id.as_str(), value));
        }
    }

    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::Compound(output),
        references,
    ))
}

fn decode_entry_source(
    descriptor: &bbb_facts::environment_attribute_types::VanillaEnvironmentAttribute,
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    if let Some(value) = context.speculate_codec_alternative(|context| {
        decode_attribute_value_source(descriptor, input, field, context)
    })? {
        return Ok(value);
    }

    context.charge_node(field)?;
    let len = input.map_len().ok_or_else(|| {
        context.codec_error(field, "attribute entry is not a direct value or map")
    })?;
    context.check_collection(field, len)?;
    context.charge_node("modifier")?;
    let modifier = required_field(input, "modifier", field, context)?
        .utf8_string()
        .ok_or_else(|| context.codec_error("modifier", "modifier is not a string"))?;
    let argument = required_field(input, "argument", field, context)?;
    let argument = context.with_nested("argument", |context| {
        decode_modifier_argument_source(descriptor, &modifier, argument, "argument", context)
    })?;

    if modifier == "override" {
        return Ok(argument);
    }
    let (argument, references) = argument.into_parts();
    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::Compound(vec![
            entry("argument", argument),
            entry("modifier", NbtSummaryValue::String(modifier.into())),
        ]),
        references,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry_codec::{
        decode_direct_registry_json, RegistryElementSource, RegistryReferenceKind,
    };
    use bbb_facts::environment_attribute_types::{
        VanillaEnvironmentModifierFamily, VANILLA_ENVIRONMENT_ATTRIBUTES,
    };
    use bbb_protocol::nbt_summary::canonical_float_tag_bits;
    use bbb_protocol::NbtSummaryEntry;

    fn decode(
        json: &[u8],
        mode: EnvironmentAttributeMapMode,
    ) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
        let data = decode_direct_registry_json(json).unwrap();
        let mut context = RegistryCodecDecodeContext::new(
            "minecraft:worldgen/biome",
            "EnvironmentAttributeMap.CODEC",
            RegistryElementSource::ServerDatapackJson,
        );
        decode_environment_attribute_map_source(data.root(), "$.attributes", &mut context, mode)
    }

    #[test]
    fn generated_inventory_drives_all_attribute_and_modifier_facts() {
        assert_eq!(VANILLA_ENVIRONMENT_ATTRIBUTES.len(), 48);
        assert_eq!(
            VANILLA_ENVIRONMENT_ATTRIBUTES
                .iter()
                .filter(|attribute| attribute.syncable)
                .count(),
            33
        );
        assert_eq!(
            VANILLA_ENVIRONMENT_ATTRIBUTES
                .iter()
                .filter(|attribute| attribute.positional)
                .count(),
            46
        );
        assert_eq!(
            vanilla_environment_attribute("minecraft:visual/fog_color")
                .unwrap()
                .value_type
                .modifier_family(),
            VanillaEnvironmentModifierFamily::Rgb
        );
    }

    #[test]
    fn direct_map_normalizes_ids_and_override_full_form_to_compact_values() {
        let decoded = decode(
            br##"{
              "visual/fog_color":"#112233",
              "minecraft:audio/firefly_bush_sounds":{
                "modifier":"override",
                "argument":true
              },
              "minecraft:gameplay/villager_activity":"idle"
            }"##,
            EnvironmentAttributeMapMode::Direct,
        )
        .unwrap();
        assert_eq!(
            decoded.network_nbt_unchecked(),
            &NbtSummaryValue::Compound(vec![
                entry(
                    "minecraft:visual/fog_color",
                    NbtSummaryValue::String("#112233".to_string().into()),
                ),
                entry(
                    "minecraft:audio/firefly_bush_sounds",
                    NbtSummaryValue::Byte(1),
                ),
                entry(
                    "minecraft:gameplay/villager_activity",
                    NbtSummaryValue::String("minecraft:idle".to_string().into()),
                ),
            ])
        );
    }

    #[test]
    fn non_override_dispatch_writes_argument_before_modifier() {
        let decoded = decode(
            br#"{
              "minecraft:visual/fog_start_distance":{
                "modifier":"add",
                "argument":2.5
              }
            }"#,
            EnvironmentAttributeMapMode::Direct,
        )
        .unwrap();
        assert_eq!(
            decoded.network_nbt_unchecked(),
            &NbtSummaryValue::Compound(vec![entry(
                "minecraft:visual/fog_start_distance",
                NbtSummaryValue::Compound(vec![
                    entry(
                        "argument",
                        NbtSummaryValue::Float(canonical_float_tag_bits(2.5)),
                    ),
                    entry(
                        "modifier",
                        NbtSummaryValue::String("add".to_string().into())
                    ),
                ]),
            )])
        );
    }

    #[test]
    fn network_filter_decodes_non_syncable_values_and_retains_their_references() {
        let decoded = decode(
            br#"{
              "minecraft:gameplay/bed_rule":{
                "can_sleep":"when_dark",
                "can_set_spawn":"always",
                "error_message":{
                  "text":"blocked",
                  "click_event":{
                    "action":"show_dialog",
                    "dialog":"example:bed_help"
                  }
                }
              }
            }"#,
            EnvironmentAttributeMapMode::Network,
        )
        .unwrap();
        assert_eq!(
            decoded.network_nbt_unchecked(),
            &NbtSummaryValue::Compound(Vec::new())
        );
        let references = decoded.references().collect::<Vec<_>>();
        assert_eq!(references.len(), 1);
        assert_eq!(references[0].target_registry, "minecraft:dialog");
        assert_eq!(references[0].kind, RegistryReferenceKind::Element);
        assert_eq!(references[0].resource_id.as_str(), "example:bed_help");

        assert!(decode(
            br#"{"minecraft:gameplay/bed_rule":{"can_sleep":"invalid"}}"#,
            EnvironmentAttributeMapMode::Network,
        )
        .is_err());
    }

    #[test]
    fn ambient_value_family_uses_particle_sound_and_compact_list_owners() {
        let decoded = decode(
            br#"{
              "minecraft:visual/ambient_particles":[{
                "particle":{"type":"minecraft:ash"},
                "probability":0.5
              }],
              "minecraft:audio/background_music":{
                "default":{
                  "sound":"minecraft:music.game",
                  "min_delay":1,
                  "max_delay":2
                }
              },
              "minecraft:audio/ambient_sounds":{
                "loop":"minecraft:ambient.cave",
                "mood":{
                  "sound":"minecraft:ambient.cave",
                  "tick_delay":6000,
                  "block_search_extent":8,
                  "offset":2.0
                },
                "additions":{
                  "sound":"minecraft:ambient.cave",
                  "tick_chance":0.01
                }
              }
            }"#,
            EnvironmentAttributeMapMode::Direct,
        )
        .unwrap();
        let NbtSummaryValue::Compound(attributes) = decoded.network_nbt_unchecked() else {
            panic!("environment map must encode as a compound");
        };
        assert_eq!(attributes.len(), 3);
        assert!(matches!(
            &attributes[0].value,
            NbtSummaryValue::List(values)
                if matches!(
                    values.as_slice(),
                    [NbtSummaryValue::Compound(fields)]
                        if fields[0].name == "particle" && fields[1].name == "probability"
                )
        ));
        assert!(matches!(
            &attributes[1].value,
            NbtSummaryValue::Compound(fields)
                if matches!(
                    fields.as_slice(),
                    [NbtSummaryEntry { name, value: NbtSummaryValue::Compound(music) }]
                        if name == "default" && music.len() == 3
                )
        ));
        assert!(matches!(
            &attributes[2].value,
            NbtSummaryValue::Compound(fields)
                if matches!(
                    fields.last(),
                    Some(NbtSummaryEntry {
                        name,
                        value: NbtSummaryValue::Compound(_),
                    }) if name == "additions"
                )
        ));
        let references = decoded.references().collect::<Vec<_>>();
        assert_eq!(references.len(), 4);
        assert!(references
            .iter()
            .all(|reference| reference.target_registry == "minecraft:sound_event"));

        assert!(decode(
            br#"{"minecraft:visual/ambient_particles":[{"particle":{"type":"ash"},"probability":-0.0}]}"#,
            EnvironmentAttributeMapMode::Direct,
        )
        .is_err());
    }
}
