//! Source-aware synchronized `Biome` migration owner.
//!
//! Server datapacks are decoded through the persistent record before
//! the retained semantic value is projected through `Biome.NETWORK_CODEC`.
//! Generation and mob settings therefore never reach the packet, but every
//! generation-local holder/tag remains attached to the candidate until the
//! frozen registry lookup authorizes publication.
//!
//! Generation holders keep their named and tag references here; inline
//! carvers and placed features are decoded by the server datapack loader.

use std::collections::HashSet;

use crate::{
    persistent_component_nbt::source_codec::{
        decode_fixed_holder_reference_source, decode_rgb_color_source, java_float_compare,
    },
    registry_codec::{
        environment_source::{
            decode_environment_attribute_map_source, EnvironmentAttributeMapMode,
        },
        recursive_values::{
            required_field, CanonicalRegistryCodecValue, OwnedRegistryCodecReference,
            RegistryCodecDecodeContext,
        },
        synchronized_registry_element_codec, RegistryCodecError, RegistryElementData,
        RegistryElementSource, RegistryElementValueRef, RegistryResourceIdentifier,
    },
};
use bbb_facts::{
    entity_types::{
        vanilla_entity_mob_category_for_serialized_name, vanilla_entity_type_for_resource_id,
    },
    environment_attribute_types::vanilla_environment_attribute,
};
use bbb_protocol::{
    content_registry::ContentRegistry,
    nbt_summary::{canonical_float_tag_bits, entry},
    NbtSummaryValue,
};

const BIOME_REGISTRY: &str = "minecraft:worldgen/biome";
#[cfg(test)]
const CONFIGURED_CARVER_REGISTRY: &str = "minecraft:worldgen/configured_carver";
#[cfg(test)]
const PLACED_FEATURE_REGISTRY: &str = "minecraft:worldgen/placed_feature";

pub(crate) fn decode_biome_element_with_content_registry(
    data: &RegistryElementData,
    content_registry: &ContentRegistry,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let codec = synchronized_registry_element_codec(BIOME_REGISTRY, data.source)?;
    let mut context = RegistryCodecDecodeContext::new(BIOME_REGISTRY, codec, data.source)
        .with_content_registry(content_registry);
    decode_biome_source(data.root(), "$", &mut context, true)
}

#[cfg(test)]
pub(crate) fn decode_biome_element(
    data: &RegistryElementData,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    decode_biome_element_with_content_registry(data, ContentRegistry::vanilla())
}

// Biome: validate the full DIRECT record, then expose its positional attribute map.
pub fn canonical_biome_environment_attributes(
    data: &RegistryElementData,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    canonical_biome_environment_attributes_with_content_registry(data, ContentRegistry::vanilla())
}

pub fn canonical_biome_environment_attributes_with_content_registry(
    data: &RegistryElementData,
    content_registry: &ContentRegistry,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    let codec = synchronized_registry_element_codec(BIOME_REGISTRY, data.source)?;
    let mut context = RegistryCodecDecodeContext::new(BIOME_REGISTRY, codec, data.source)
        .with_content_registry(content_registry);
    let decoded = decode_biome_source(data.root(), "$", &mut context, false)?;
    let (NbtSummaryValue::Compound(fields), _) = decoded.into_parts() else {
        unreachable!("Biome record decoder emits a compound");
    };
    Ok(fields
        .into_iter()
        .find(|entry| entry.name == "attributes")
        .map_or_else(
            || NbtSummaryValue::Compound(Vec::new()),
            |entry| entry.value,
        ))
}

fn decode_biome_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    filter_non_syncable: bool,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    // `Biome.DIRECT_CODEC` owns climate, positional attributes, effects,
    // generation and mob settings. `NETWORK_CODEC` owns only the first three
    // and filters attributes after decoding them.
    context.charge_record(input, field, "Biome record")?;

    let mut output = Vec::new();
    let mut references = Vec::new();
    let has_precipitation = required_bool_field(input, "has_precipitation", context)?;
    output.push(entry(
        "has_precipitation",
        NbtSummaryValue::Byte(i8::from(has_precipitation)),
    ));

    let temperature = required_f32_field(input, "temperature", context)?;
    output.push(entry(
        "temperature",
        NbtSummaryValue::Float(canonical_float_tag_bits(temperature)),
    ));

    // The serialized enum contains `none` and `frozen`; `none` is the codec
    // default and is omitted by the writer.
    let temperature_modifier =
        optional_enum_field(input, "temperature_modifier", &["none", "frozen"], context)?;
    if let Some(temperature_modifier) = temperature_modifier.filter(|value| *value != "none") {
        output.push(entry(
            "temperature_modifier",
            NbtSummaryValue::String(temperature_modifier.into()),
        ));
    }

    let downfall = required_f32_field(input, "downfall", context)?;
    output.push(entry(
        "downfall",
        NbtSummaryValue::Float(canonical_float_tag_bits(downfall)),
    ));

    if let Some(attributes_input) = optional_field(input, "attributes", context)? {
        let attributes = context.with_nested("attributes", |context| {
            if context.source() == RegistryElementSource::ServerDatapackJson {
                // DIRECT first applies CODEC_ONLY_POSITIONAL. Reusing that
                // decoded value and filtering its canonical entries produces
                // the network map without decoding recursive values twice.
                decode_environment_attribute_map_source(
                    attributes_input,
                    "attributes",
                    context,
                    EnvironmentAttributeMapMode::PositionalDirect,
                )
                .map(|attributes| {
                    if filter_non_syncable {
                        filter_syncable_attributes(attributes)
                    } else {
                        attributes
                    }
                })
            } else {
                decode_environment_attribute_map_source(
                    attributes_input,
                    "attributes",
                    context,
                    EnvironmentAttributeMapMode::Network,
                )
            }
        })?;
        let (attributes, attribute_references) = attributes.into_parts();
        if !is_empty_compound(&attributes) {
            output.push(entry("attributes", attributes));
        }
        references.extend(attribute_references);
    }

    let effects_input = required_field(input, "effects", field, context)?;
    let effects = context.with_nested("effects", |context| {
        decode_biome_effects(effects_input, "effects", context)
    })?;
    output.push(entry("effects", effects));

    if context.source() == RegistryElementSource::ServerDatapackJson {
        // `BiomeGenerationSettings.CODEC` requires both fields and retains
        // the configured-carver and placed-feature holder sets
        references.extend(decode_generation_settings(input, context)?);

        // `MobSpawnSettings.CODEC` is part of DIRECT_CODEC even though none of
        // it is synchronized.
        decode_mob_spawn_settings(input, context)?;
    }

    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::Compound(output),
        references,
    ))
}

fn filter_syncable_attributes(decoded: CanonicalRegistryCodecValue) -> CanonicalRegistryCodecValue {
    let (network_nbt, references) = decoded.into_parts();
    let NbtSummaryValue::Compound(entries) = network_nbt else {
        unreachable!("environment attribute owner always emits a compound")
    };
    let entries = entries
        .into_iter()
        .filter(|entry| {
            entry
                .name
                .with_utf8(vanilla_environment_attribute)
                .flatten()
                .is_some_and(|attribute| attribute.syncable)
        })
        .collect();
    CanonicalRegistryCodecValue::from_parts(NbtSummaryValue::Compound(entries), references)
}

fn decode_biome_effects(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    // Field order and defaults are the `BiomeSpecialEffects.CODEC` record
    // order.
    context.charge_record(input, field, "BiomeSpecialEffects record")?;

    let water_color = required_rgb_color_field(input, "water_color", context)?;
    let mut output = vec![entry(
        "water_color",
        NbtSummaryValue::String(format!("#{water_color:06x}").into()),
    )];

    for name in ["foliage_color", "dry_foliage_color", "grass_color"] {
        if let Some(color) = optional_rgb_color_field(input, name, context)? {
            output.push(entry(
                name,
                NbtSummaryValue::String(format!("#{color:06x}").into()),
            ));
        }
    }

    let modifier = optional_enum_field(
        input,
        "grass_color_modifier",
        &["none", "dark_forest", "swamp"],
        context,
    )?;
    if let Some(modifier) = modifier.filter(|value| *value != "none") {
        output.push(entry(
            "grass_color_modifier",
            NbtSummaryValue::String(modifier.into()),
        ));
    }

    Ok(NbtSummaryValue::Compound(output))
}

fn decode_generation_settings(
    input: RegistryElementValueRef<'_>,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<Vec<OwnedRegistryCodecReference>, RegistryCodecError> {
    let mut references = Vec::new();
    let carvers = required_field(input, "carvers", "carvers", context)?;
    generation_holder_set(
        carvers,
        "carvers",
        context,
        "minecraft:worldgen/configured_carver",
        false,
        &mut references,
    )?;

    // `PlacedFeature.LIST_OF_LISTS_CODEC` is always-list inner holder sets.
    let features_input = required_field(input, "features", "features", context)?;
    context.charge_node("features")?;
    let feature_steps = features_input
        .list_elements()
        .ok_or_else(|| context.codec_error("features", "features are not a list"))?;
    context.check_collection("features", feature_steps.len())?;
    for step in feature_steps {
        generation_holder_set(
            step,
            "features",
            context,
            "minecraft:worldgen/placed_feature",
            true,
            &mut references,
        )?;
    }
    Ok(references)
}

/// `RegistryCodecs.homogeneousList` over a worldgen registry: a `#tag`, one
/// holder (unless `always_use_list`), or a list of all-named or all-inline
/// holders. Inline holders are left to the datapack decoder.
fn generation_holder_set(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    registry: &'static str,
    always_use_list: bool,
    references: &mut Vec<OwnedRegistryCodecReference>,
) -> Result<(), RegistryCodecError> {
    if let Some(raw) = input
        .java_string_ref()
        .and_then(|raw| raw.units().strip_prefix(&[b'#' as u16]))
        .and_then(|units| String::from_utf16(units).ok())
    {
        context.charge_node(field)?;
        let identifier = RegistryResourceIdentifier::parse_owned(raw)
            .ok_or_else(|| context.codec_error(field, "invalid tag identifier"))?;
        references.push(OwnedRegistryCodecReference::tag(registry, identifier));
        return Ok(());
    }
    let mut holder = |item: RegistryElementValueRef<'_>,
                      context: &mut RegistryCodecDecodeContext<'_>|
     -> Result<bool, RegistryCodecError> {
        if item
            .java_string_ref()
            .is_some_and(|raw| raw.with_utf8(|_| ()).is_some())
        {
            let decoded = decode_fixed_holder_reference_source(item, field, context, registry)?;
            references.extend(decoded.into_parts().1);
            return Ok(true);
        }
        context.charge_node(field)?;
        if item.map_len().is_none() {
            return Err(context.codec_error(field, "expected a holder id or inline value"));
        }
        Ok(false)
    };
    let Some(items) = input.list_elements() else {
        if always_use_list {
            return Err(context.codec_error(field, "expected a holder list or #tag"));
        }
        return holder(input, context).map(|_| ());
    };
    context.charge_node(field)?;
    context.check_collection(field, items.len())?;
    let mut first_named = None;
    for item in items {
        let named = holder(item, context)?;
        if *first_named.get_or_insert(named) != named {
            return Err(context.codec_error(field, "Mixed type list"));
        }
    }
    Ok(())
}

fn decode_mob_spawn_settings(
    input: RegistryElementValueRef<'_>,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    if let Some(probability) = optional_field(input, "creature_spawn_probability", context)? {
        let probability = context.decode_f32(probability, "creature_spawn_probability")?;
        if java_float_compare(probability, 0.0).is_lt()
            || java_float_compare(probability, 0.999_999_9).is_gt()
        {
            return Err(context.codec_error(
                "creature_spawn_probability",
                "float is outside 0.0..=0.9999999",
            ));
        }
    }

    let spawners = required_field(input, "spawners", "spawners", context)?;
    decode_spawners(spawners, "spawners", context)?;

    let costs = required_field(input, "spawn_costs", "spawn_costs", context)?;
    decode_spawn_costs(costs, "spawn_costs", context)
}

fn decode_spawners(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    let len = context.charge_record(input, field, "spawner map")?;
    let visible = input.visible_map_entries(len);
    for (category, weighted_list) in visible {
        if category
            .with_utf8(vanilla_entity_mob_category_for_serialized_name)
            .flatten()
            .is_none()
        {
            return Err(context.codec_error(field, format!("unknown mob category `{category}`")));
        }
        context.charge_node(field)?;
        let items = weighted_list
            .list_elements()
            .ok_or_else(|| context.codec_error(field, "spawner category is not a weighted list"))?;
        context.check_collection(field, items.len())?;
        let mut total_weight = 0_i64;
        for item in items {
            let weight =
                context.with_nested(field, |context| decode_spawner_data(item, field, context))?;
            total_weight += i64::from(weight);
            if total_weight > i64::from(i32::MAX) {
                return Err(context.codec_error(field, "sum of spawner weights exceeds 2147483647"));
            }
        }
    }
    Ok(())
}

fn decode_spawner_data(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<i32, RegistryCodecError> {
    context.charge_record(input, field, "SpawnerData record")?;
    decode_builtin_entity_type(
        required_field(input, "type", field, context)?,
        "type",
        context,
    )?;
    let min = required_i32_range_field(input, "minCount", context, 1, i32::MAX)?;
    let max = required_i32_range_field(input, "maxCount", context, 1, i32::MAX)?;
    if min > max {
        return Err(context.codec_error(field, "minCount needs to be smaller or equal to maxCount"));
    }
    required_i32_range_field(input, "weight", context, 0, i32::MAX)
}

fn decode_spawn_costs(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    let len = context.charge_record(input, field, "spawn-cost map")?;
    let visible = input.visible_map_entries(len);
    let mut identities = HashSet::with_capacity(visible.len());
    for (raw_type, value) in visible {
        let identifier =
            RegistryResourceIdentifier::parse_java_string(raw_type).ok_or_else(|| {
                context.codec_error(field, format!("invalid entity type `{raw_type}`"))
            })?;
        if !identities.insert(identifier.as_str().to_string()) {
            return Err(context.codec_error(
                field,
                format!("duplicate entity type {}", identifier.as_str()),
            ));
        }
        if vanilla_entity_type_for_resource_id(identifier.as_str()).is_none()
            && context
                .content_registry()
                .entity_by_resource_id(identifier.as_str())
                .is_none()
        {
            return Err(context.codec_error(
                field,
                format!("unknown entity type {}", identifier.as_str()),
            ));
        }
        context.with_nested(field, |context| {
            context.charge_record(value, field, "MobSpawnCost record")?;
            required_f64_field(value, "energy_budget", context)?;
            required_f64_field(value, "charge", context)?;
            Ok(())
        })?;
    }
    Ok(())
}

fn decode_builtin_entity_type(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    context.charge_node(field)?;
    let raw = input
        .java_string_ref()
        .ok_or_else(|| context.codec_error(field, "entity type is not a string"))?;
    let identifier = RegistryResourceIdentifier::parse_java_string(raw)
        .ok_or_else(|| context.codec_error(field, format!("invalid entity type `{raw}`")))?;
    if vanilla_entity_type_for_resource_id(identifier.as_str()).is_none()
        && context
            .content_registry()
            .entity_by_resource_id(identifier.as_str())
            .is_none()
    {
        return Err(context.codec_error(
            field,
            format!("unknown entity type {}", identifier.as_str()),
        ));
    }
    Ok(())
}

fn required_rgb_color_field(
    input: RegistryElementValueRef<'_>,
    name: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<u32, RegistryCodecError> {
    let value = required_field(input, name, name, context)?;
    decode_rgb_color(value, name, context)
}

fn optional_rgb_color_field(
    input: RegistryElementValueRef<'_>,
    name: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<Option<u32>, RegistryCodecError> {
    let Some(value) = optional_field(input, name, context)? else {
        return Ok(None);
    };
    decode_rgb_color(value, name, context).map(Some)
}

fn decode_rgb_color(
    value: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<u32, RegistryCodecError> {
    // STRING_RGB_COLOR tries the strict `#`-prefixed six-digit codec, then
    // RGB_COLOR_CODEC's integer / fixed Vector3f alternatives. Its writer is
    // always the lowercase string form.
    if let Some(raw) = value.utf8_string() {
        context.charge_node(field)?;
        let digits = raw
            .strip_prefix('#')
            .filter(|digits| {
                digits.len() == 6 && digits.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
            .ok_or_else(|| {
                context.codec_error(field, "hex RGB color must be # followed by six digits")
            })?;
        return u32::from_str_radix(digits, 16)
            .map_err(|_| context.codec_error(field, "invalid hexadecimal RGB color"));
    }
    decode_rgb_color_source(value, field, context).map(|value| value as u32 & 0x00ff_ffff)
}

fn optional_field<'a>(
    input: RegistryElementValueRef<'a>,
    name: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> Result<Option<RegistryElementValueRef<'a>>, RegistryCodecError> {
    match input.record_field(name) {
        None => Ok(None),
        Some(value) if value.is_null() => {
            Err(context.codec_error(name, format!("optional field `{name}` must not be null")))
        }
        Some(value) => Ok(Some(value)),
    }
}

fn required_bool_field(
    input: RegistryElementValueRef<'_>,
    name: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<bool, RegistryCodecError> {
    let value = required_field(input, name, name, context)?;
    context.decode_bool(value, name)
}

fn required_f32_field(
    input: RegistryElementValueRef<'_>,
    name: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<f32, RegistryCodecError> {
    let value = required_field(input, name, name, context)?;
    context.decode_f32(value, name)
}

fn required_f64_field(
    input: RegistryElementValueRef<'_>,
    name: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<f64, RegistryCodecError> {
    let value = required_field(input, name, name, context)?;
    context.decode_f64(value, name)
}

fn required_i32_range_field(
    input: RegistryElementValueRef<'_>,
    name: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    minimum: i32,
    maximum: i32,
) -> Result<i32, RegistryCodecError> {
    let value = required_field(input, name, name, context)?;
    context.decode_i32_range(value, name, minimum, maximum)
}

fn optional_enum_field<'a>(
    input: RegistryElementValueRef<'a>,
    name: &str,
    allowed: &'a [&'a str],
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<Option<&'a str>, RegistryCodecError> {
    let Some(value) = optional_field(input, name, context)? else {
        return Ok(None);
    };
    context.charge_node(name)?;
    let raw = value
        .java_string_ref()
        .ok_or_else(|| context.codec_error(name, "expected a string enum"))?;
    allowed
        .iter()
        .copied()
        .find(|candidate| raw == candidate)
        .map(Some)
        .ok_or_else(|| context.codec_error(name, format!("unknown enum value `{raw}`")))
}

fn is_empty_compound(value: &NbtSummaryValue) -> bool {
    matches!(value, NbtSummaryValue::Compound(values) if values.is_empty())
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use serde_json::{json, Value};

    use super::*;
    use crate::registry_codec::{
        decode_direct_registry_json, RegistryCodecReference, RegistryReferenceKind,
    };

    fn valid_biome() -> Value {
        json!({
            "has_precipitation": true,
            "temperature": 0.8,
            "temperature_modifier": "none",
            "downfall": 0.4,
            "attributes": {"minecraft:visual/sky_color": "#78A7FF"},
            "effects": {
                "water_color": "#3F76E4",
                "grass_color_modifier": "none"
            },
            "carvers": ["minecraft:cave", "minecraft:canyon"],
            "features": [[], ["minecraft:lake_lava_surface"]],
            "creature_spawn_probability": 0.1,
            "spawners": {
                "creature": [{
                    "type": "minecraft:pig",
                    "minCount": 1,
                    "maxCount": 4,
                    "weight": 10
                }],
                "monster": []
            },
            "spawn_costs": {
                "minecraft:zombie": {"energy_budget": 1.0, "charge": 0.25}
            }
        })
    }

    #[test]
    fn direct_biome_environment_retains_server_modifiers_without_network_leakage() {
        let mut biome = valid_biome();
        biome["attributes"]["gameplay/can_pillager_patrol_spawn"] =
            json!({"modifier":"and","argument":false});
        let attributes = canonical_biome_environment_attributes(
            &decode_direct_registry_json(&serde_json::to_vec(&biome).unwrap()).unwrap(),
        )
        .unwrap();
        let NbtSummaryValue::Compound(attributes) = attributes else {
            panic!("attributes");
        };
        assert!(attributes.iter().any(|attribute| attribute
            == &entry(
                "minecraft:gameplay/can_pillager_patrol_spawn",
                NbtSummaryValue::Compound(vec![
                    entry("argument", NbtSummaryValue::Byte(0)),
                    entry("modifier", NbtSummaryValue::String("and".into()))
                ]),
            )));
        let network = decode(&biome).unwrap();
        let NbtSummaryValue::Compound(fields) = network.network_nbt_unchecked() else {
            panic!("biome");
        };
        let NbtSummaryValue::Compound(attributes) = &fields
            .iter()
            .find(|field| field.name == "attributes")
            .unwrap()
            .value
        else {
            panic!("attributes");
        };
        assert_eq!(attributes.len(), 1);
        assert_eq!(attributes[0].name, "minecraft:visual/sky_color");
        biome.as_object_mut().unwrap().remove("attributes");
        assert_eq!(
            canonical_biome_environment_attributes(
                &decode_direct_registry_json(&serde_json::to_vec(&biome).unwrap()).unwrap(),
            )
            .unwrap(),
            NbtSummaryValue::Compound(vec![])
        );
    }

    fn decode(value: &Value) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
        let bytes = serde_json::to_vec(value).unwrap();
        let data = decode_direct_registry_json(&bytes).unwrap();
        decode_biome_element(&data)
    }

    fn references(value: &CanonicalRegistryCodecValue) -> Vec<RegistryCodecReference<'_>> {
        value.references().collect()
    }

    #[test]
    fn direct_validation_projects_network_shape_and_retains_named_generation_references() {
        let decoded = decode(&valid_biome()).unwrap();
        let NbtSummaryValue::Compound(fields) = decoded.network_nbt_unchecked() else {
            panic!("biome network value must be a compound")
        };
        assert_eq!(
            fields.iter().map(|entry| &entry.name).collect::<Vec<_>>(),
            [
                "has_precipitation",
                "temperature",
                "downfall",
                "attributes",
                "effects"
            ]
        );
        assert!(matches!(
            &fields[3].value,
            NbtSummaryValue::Compound(attributes)
                if attributes[0].name == "minecraft:visual/sky_color"
                    && attributes[0].value == NbtSummaryValue::String("#78a7ff".to_string().into())
        ));
        assert!(matches!(
            &fields[4].value,
            NbtSummaryValue::Compound(effects)
                if effects == &[entry("water_color", NbtSummaryValue::String("#3f76e4".to_string().into()))]
        ));
        assert_eq!(
            references(&decoded)
                .into_iter()
                .map(|reference| (
                    reference.target_registry,
                    reference.kind,
                    reference.resource_id.as_str(),
                ))
                .collect::<Vec<_>>(),
            [
                (
                    CONFIGURED_CARVER_REGISTRY,
                    RegistryReferenceKind::Element,
                    "minecraft:cave"
                ),
                (
                    CONFIGURED_CARVER_REGISTRY,
                    RegistryReferenceKind::Element,
                    "minecraft:canyon"
                ),
                (
                    PLACED_FEATURE_REGISTRY,
                    RegistryReferenceKind::Element,
                    "minecraft:lake_lava_surface"
                ),
            ]
        );
    }

    #[test]
    fn repo_managed_builtin_biome_corpus_direct_decodes() {
        let Some(root) = std::env::var_os("BBB_VANILLA_ROOT") else {
            eprintln!("skipping built-in biome corpus; BBB_VANILLA_ROOT is unset");
            return;
        };
        let directory = PathBuf::from(root).join("sources/26.1/data/minecraft/worldgen/biome");
        let mut paths = fs::read_dir(&directory)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", directory.display()))
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "json")
            })
            .collect::<Vec<_>>();
        paths.sort();
        assert_eq!(paths.len(), 65);
        for path in paths {
            let json = fs::read(&path).unwrap();
            let data = decode_direct_registry_json(&json)
                .unwrap_or_else(|error| panic!("{} source JSON failed: {error}", path.display()));
            decode_biome_element(&data)
                .unwrap_or_else(|error| panic!("{} direct decode failed: {error}", path.display()));
        }
    }
}
