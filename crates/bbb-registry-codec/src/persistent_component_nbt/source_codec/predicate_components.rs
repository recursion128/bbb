//! Source-aware predicate component codecs.
//!
//! This module owns `AdventureModePredicate.CODEC` for `can_place_on` and
//! `can_break`, plus the `ItemPredicate.CODEC` wrapped by `LockCode`.  The
//! nested component matcher map stays in the recursive registry-codec family:
//! exact values call the central component owner and holder sets retain frozen
//! registry references rather than resolving them while decoding.
//!
//! Vanilla 26.1 sources: `AdventureModePredicate.java:24-31`, criterion
//! criterion `BlockPredicate`, `ItemPredicate`,
//! `DataComponentPredicate`, and

use std::collections::HashSet;

pub(super) use crate::registry_codec::recursive_values::decode_list;

use crate::registry_codec::{
    recursive_values::{
        required_field, CanonicalRegistryCodecValue, RegistryCodecDecodeContext,
        RegistryCodecRecord,
    },
    source_nbt::decode_lenient_flattened_compound_source,
    RegistryCodecError, RegistryElementValueRef, RegistryResourceIdentifier,
};
use bbb_facts::data_component_types::{self};
use bbb_protocol::{nbt_summary::canonical_double_tag_bits, NbtSummaryValue};

use super::holders::decode_homogeneous_fixed_holder_set;
use super::{decode_component_value_source, normalize_component_identifier};
#[cfg(test)]
use bbb_protocol::nbt_summary::entry;

pub(super) mod criterion;
mod partial;

const BLOCK_REGISTRY: &str = "minecraft:block";
const ITEM_REGISTRY: &str = "minecraft:item";

/// Criterion `BlockPredicate`: `blocks`, `state`, `nbt`, then flattened
/// `components` and `predicates`, in `RecordCodecBuilder` declaration order.
fn decode_block_predicate(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "block predicate")?;
    let mut output = RegistryCodecRecord::default();

    if let Some(blocks) = input.field("blocks") {
        let child = child(field, "blocks");
        let decoded = context.with_nested(&child, |context| {
            decode_homogeneous_fixed_holder_set(blocks, &child, context, BLOCK_REGISTRY, false)
        })?;
        output.push("blocks", decoded);
    }
    if let Some(state) = input.field("state") {
        let child = child(field, "state");
        let decoded = context.with_nested(&child, |context| {
            decode_state_properties_predicate(state, &child, context)
        })?;
        output.push("state", decoded);
    }
    if let Some(nbt) = input.field("nbt") {
        let child = child(field, "nbt");
        let decoded = context.with_nested(&child, |context| {
            decode_lenient_flattened_compound_source(nbt, &child, context)
        })?;
        output.push("nbt", decoded);
    }
    let matchers = decode_data_component_matchers(input, field, context)?;
    output.append(matchers);
    Ok(output.finish())
}

/// `ItemPredicate`, also the complete value codec wrapped by `LockCode`.
pub(crate) fn decode_item_predicate(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "item predicate")?;
    let mut output = RegistryCodecRecord::default();

    if let Some(items) = input.field("items") {
        let child = child(field, "items");
        let decoded = context.with_nested(&child, |context| {
            decode_homogeneous_fixed_holder_set(items, &child, context, ITEM_REGISTRY, false)
        })?;
        output.push("items", decoded);
    }
    if let Some(count) = input.field("count") {
        let child = child(field, "count");
        let decoded =
            context.with_nested(&child, |context| decode_int_bounds(count, &child, context))?;
        if !decoded.is_any {
            output.push("count", decoded.value);
        }
    }
    let matchers = decode_data_component_matchers(input, field, context)?;
    output.append(matchers);
    Ok(output.finish())
}

fn decode_data_component_matchers(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<RegistryCodecRecord, RegistryCodecError> {
    let mut output = RegistryCodecRecord::default();
    if let Some(components) = input.field("components") {
        let child = child(field, "components");
        let decoded = context.with_nested(&child, |context| {
            decode_exact_components(components, &child, context)
        })?;
        if !decoded.is_empty {
            output.push("components", decoded.value);
        }
    }
    if let Some(predicates) = input.field("predicates") {
        let child = child(field, "predicates");
        let decoded = context.with_nested(&child, |context| {
            decode_predicates_map(predicates, &child, context)
        })?;
        if !decoded.is_empty {
            output.push("predicates", decoded.value);
        }
    }
    Ok(output)
}

struct DecodedMap {
    value: CanonicalRegistryCodecValue,
    is_empty: bool,
}

/// `DataComponentExactPredicate`: no removal keys and no transient values.
fn decode_exact_components(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<DecodedMap, RegistryCodecError> {
    let len = context.charge_record(input, field, "exact component map")?;
    let visible = input.visible_map_entries(len);
    let is_empty = visible.is_empty();
    let mut normalized = HashSet::with_capacity(visible.len());
    let mut output = RegistryCodecRecord::default();
    for (raw_key, value) in visible {
        let item_field = child(field, raw_key);
        let resource_id = raw_key
            .with_utf8(|raw| normalize_component_identifier(raw, &item_field, context))
            .unwrap_or_else(|| {
                Err(context.codec_error(&item_field, "invalid component identifier"))
            })?;
        if !normalized.insert(resource_id.clone()) {
            return Err(context.codec_error(
                &item_field,
                format!("duplicate normalized data component {resource_id}"),
            ));
        }
        let descriptor =
            data_component_types::vanilla_data_component_type_for_resource_id(&resource_id)
                .filter(|descriptor| descriptor.persistent)
                .ok_or_else(|| {
                    context.codec_error(
                        &item_field,
                        format!("unknown persistent data component {resource_id}"),
                    )
                })?;
        let decoded = context.with_nested(&item_field, |context| {
            decode_component_value_source(descriptor, value, &item_field, context)
        })?;
        output.push(&resource_id, decoded);
    }
    Ok(DecodedMap {
        value: output.finish(),
        is_empty,
    })
}

/// `DataComponentPredicate.Type.CODEC` tries the concrete predicate registry
/// before the data-component registry. This ordering makes a concrete key win
/// when both registries contain the same resource id.
fn decode_predicates_map(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<DecodedMap, RegistryCodecError> {
    let len = context.charge_record(input, field, "component predicate map")?;
    let visible = input.visible_map_entries(len);
    let is_empty = visible.is_empty();
    let mut normalized = HashSet::with_capacity(visible.len());
    let mut entries = Vec::new();
    for (raw_key, value) in visible {
        let item_field = child(field, raw_key);
        let identifier =
            RegistryResourceIdentifier::parse_java_string(raw_key).ok_or_else(|| {
                context.codec_error(
                    &item_field,
                    format!("invalid data component predicate id {raw_key}"),
                )
            })?;
        let resource_id = identifier.into_string();
        let decoded = if let Some(predicate_type) = predicate_type_for_resource_id(&resource_id) {
            if !normalized.insert(resource_id.clone()) {
                return Err(context.codec_error(
                    &item_field,
                    format!("duplicate normalized data component predicate {resource_id}"),
                ));
            }
            context.with_nested(&item_field, |context| {
                partial::decode_concrete_predicate(
                    predicate_type.type_id,
                    value,
                    &item_field,
                    context,
                )
            })?
        } else if data_component_types::vanilla_data_component_type_for_resource_id(&resource_id)
            .is_some()
        {
            context.with_nested(&item_field, |context| {
                decode_any_value(value, &item_field, context)
            })?
        } else {
            return Err(context.codec_error(
                &item_field,
                format!("unknown data component predicate type {resource_id}"),
            ));
        };
        entries.push((resource_id, decoded));
    }
    Ok(DecodedMap {
        value: encode_matcher_map(entries),
        is_empty,
    })
}

fn encode_matcher_map(
    entries: Vec<(String, CanonicalRegistryCodecValue)>,
) -> CanonicalRegistryCodecValue {
    let mut positions = std::collections::HashMap::new();
    let mut visible = Vec::<(String, CanonicalRegistryCodecValue)>::new();
    for (name, value) in entries {
        if let Some(&position) = positions.get(&name) {
            visible[position] = (name, value);
        } else {
            positions.insert(name.clone(), visible.len());
            visible.push((name, value));
        }
    }
    let mut output = RegistryCodecRecord::default();
    for (name, value) in visible {
        output.push(&name, value);
    }
    output.finish()
}

fn predicate_type_for_resource_id(
    resource_id: &str,
) -> Option<&'static data_component_types::DataComponentPredicateTypeCatalogEntry> {
    data_component_types::VANILLA_DATA_COMPONENT_PREDICATE_TYPES
        .iter()
        .find(|entry| entry.resource_id == resource_id)
}

fn decode_any_value(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "any-value predicate payload")?;
    Ok(empty_object())
}

/// Unbounded property-name map whose values are exact strings or ranged
/// `{min?,max?}` strings. `Either` encodes an equal range through its first
/// (exact-string) branch.
pub(crate) fn decode_state_properties_predicate(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let len = context.charge_record(input, field, "state predicate")?;
    let visible = input.visible_map_entries(len);
    let mut output = RegistryCodecRecord::default();
    for (name, matcher) in visible {
        let matcher_field = child(field, name.to_display_string());
        let decoded = context.with_nested(&matcher_field, |context| {
            if matcher.java_string().is_some() {
                decode_property_string(matcher, &matcher_field, context, "property value")
            } else {
                context.charge_record(matcher, &matcher_field, "property range")?;
                let min_field = child(&matcher_field, "min");
                let min = matcher
                    .field("min")
                    .map(|value| {
                        context.with_nested(&min_field, |context| {
                            decode_property_string(value, &min_field, context, "property minimum")
                        })
                    })
                    .transpose()?;
                let max_field = child(&matcher_field, "max");
                let max = matcher
                    .field("max")
                    .map(|value| {
                        context.with_nested(&max_field, |context| {
                            decode_property_string(value, &max_field, context, "property maximum")
                        })
                    })
                    .transpose()?;
                let mut range = RegistryCodecRecord::default();
                if let Some(min) = min {
                    range.push("min", min);
                }
                if let Some(max) = max {
                    range.push("max", max);
                }
                Ok(range.finish())
            }
        })?;
        output.push(name, decoded);
    }
    Ok(output.finish())
}

fn decode_property_string(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    expected: &str,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    let value = input
        .java_string()
        .ok_or_else(|| context.codec_error(field, format!("expected {expected} string")))?;
    Ok(CanonicalRegistryCodecValue::new(value.to_nbt()))
}

fn empty_object() -> CanonicalRegistryCodecValue {
    RegistryCodecRecord::default().finish()
}

pub(super) fn child(field: &str, name: impl std::fmt::Display) -> String {
    format!("{field}.{name}")
}

pub(super) fn decode_string(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    expected: &str,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    let value = input
        .java_string()
        .ok_or_else(|| context.codec_error(field, format!("expected {expected} string")))?;
    Ok(CanonicalRegistryCodecValue::new(
        NbtSummaryValue::string_from_utf16_slice(value.units()),
    ))
}

pub(super) fn decode_identifier_value(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    let raw = input
        .java_string_ref()
        .ok_or_else(|| context.codec_error(field, "expected resource identifier string"))?;
    let identifier = RegistryResourceIdentifier::parse_java_string(raw)
        .ok_or_else(|| context.codec_error(field, format!("invalid resource identifier {raw}")))?;
    let value = identifier.into_string();
    Ok(CanonicalRegistryCodecValue::new(NbtSummaryValue::String(
        value.into(),
    )))
}

pub(super) fn decode_bool_value(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    let value = input
        .as_bool()
        .ok_or_else(|| context.codec_error(field, "expected Codec.BOOL input"))?;
    Ok(CanonicalRegistryCodecValue::new(NbtSummaryValue::Byte(
        i8::from(value),
    )))
}

pub(super) fn decode_named_value(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    expected: &str,
    accepts: impl FnOnce(&str) -> bool,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let value = decode_string(input, field, context, expected)?;
    let name = match value.network_nbt_unchecked() {
        NbtSummaryValue::String(name) => name,
        _ => return Err(context.codec_error(field, format!("unknown {expected}"))),
    };
    if !name.with_utf8(accepts).unwrap_or(false) {
        return Err(context.codec_error(field, format!("unknown {expected} {name}")));
    }
    Ok(value)
}

pub(super) struct DecodedBounds {
    pub(super) value: CanonicalRegistryCodecValue,
    pub(super) is_any: bool,
}

/// `MinMaxBounds.Ints.CODEC`: scalar points, object ranges, checked swapped
/// bounds, and scalar writer compaction for equal endpoints.
pub(super) fn decode_int_bounds(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<DecodedBounds, RegistryCodecError> {
    if !input.is_compound() {
        let point = decode_i32_value(input, field, context)?;
        return Ok(DecodedBounds {
            value: point,
            is_any: false,
        });
    }
    context.charge_record(input, field, "integer bounds")?;
    let min_field = child(field, "min");
    let min = input
        .field("min")
        .map(|value| {
            context.with_nested(&min_field, |context| {
                decode_i32_value(value, &min_field, context)
            })
        })
        .transpose()?;
    let max_field = child(field, "max");
    let max = input
        .field("max")
        .map(|value| {
            context.with_nested(&max_field, |context| {
                decode_i32_value(value, &max_field, context)
            })
        })
        .transpose()?;
    encode_int_bounds(min, max, field, context)
}

fn encode_int_bounds(
    min: Option<CanonicalRegistryCodecValue>,
    max: Option<CanonicalRegistryCodecValue>,
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> Result<DecodedBounds, RegistryCodecError> {
    let min_value = min.as_ref().map(canonical_i32);
    let max_value = max.as_ref().map(canonical_i32);
    if min_value.zip(max_value).is_some_and(|(min, max)| min > max) {
        return Err(context.codec_error(field, "swapped bounds: min is higher than max"));
    }
    if min_value.is_some() && min_value == max_value {
        return Ok(DecodedBounds {
            value: min.expect("equal point has minimum"),
            is_any: false,
        });
    }
    let is_any = min.is_none() && max.is_none();
    let mut output = RegistryCodecRecord::default();
    if let Some(min) = min {
        output.push("min", min);
    }
    if let Some(max) = max {
        output.push("max", max);
    }
    Ok(DecodedBounds {
        value: output.finish(),
        is_any,
    })
}

pub(super) fn decode_double_bounds(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<DecodedBounds, RegistryCodecError> {
    if !input.is_compound() {
        let point = decode_f64_value(input, field, context)?;
        return Ok(DecodedBounds {
            value: point.value,
            is_any: false,
        });
    }
    context.charge_record(input, field, "double bounds")?;
    let min_field = child(field, "min");
    let min = input
        .field("min")
        .map(|value| {
            context.with_nested(&min_field, |context| {
                decode_f64_value(value, &min_field, context)
            })
        })
        .transpose()?;
    let max_field = child(field, "max");
    let max = input
        .field("max")
        .map(|value| {
            context.with_nested(&max_field, |context| {
                decode_f64_value(value, &max_field, context)
            })
        })
        .transpose()?;
    encode_double_bounds(min, max, field, context)
}

fn encode_double_bounds(
    min: Option<DecodedF64Value>,
    max: Option<DecodedF64Value>,
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> Result<DecodedBounds, RegistryCodecError> {
    let min_value = min.as_ref().map(|value| value.number);
    let max_value = max.as_ref().map(|value| value.number);
    if min_value
        .zip(max_value)
        .is_some_and(|(min, max)| java_double_compare(min, max).is_gt())
    {
        return Err(context.codec_error(field, "swapped bounds: min is higher than max"));
    }
    if min_value
        .zip(max_value)
        .is_some_and(|(min, max)| java_double_compare(min, max).is_eq())
    {
        return Ok(DecodedBounds {
            value: min.expect("equal point has minimum").value,
            is_any: false,
        });
    }
    let is_any = min.is_none() && max.is_none();
    let mut output = RegistryCodecRecord::default();
    if let Some(min) = min {
        output.push("min", min.value);
    }
    if let Some(max) = max {
        output.push("max", max.value);
    }
    Ok(DecodedBounds {
        value: output.finish(),
        is_any,
    })
}

fn decode_i32_value(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    let value = input
        .codec_i32()
        .ok_or_else(|| context.codec_error(field, "expected Codec.INT input"))?;
    Ok(CanonicalRegistryCodecValue::new(NbtSummaryValue::Int(
        value,
    )))
}

struct DecodedF64Value {
    value: CanonicalRegistryCodecValue,
    number: f64,
}

fn decode_f64_value(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<DecodedF64Value, RegistryCodecError> {
    context.charge_node(field)?;
    let value = input
        .codec_f64()
        .ok_or_else(|| context.codec_error(field, "expected Codec.DOUBLE input"))?;
    Ok(DecodedF64Value {
        value: CanonicalRegistryCodecValue::new(NbtSummaryValue::Double(
            canonical_double_tag_bits(value),
        )),
        number: value,
    })
}

fn canonical_i32(value: &CanonicalRegistryCodecValue) -> i32 {
    match value.network_nbt_unchecked() {
        NbtSummaryValue::Int(value) => *value,
        _ => unreachable!("integer decoder always emits Int"),
    }
}

fn java_double_compare(left: f64, right: f64) -> std::cmp::Ordering {
    if left < right {
        return std::cmp::Ordering::Less;
    }
    if left > right {
        return std::cmp::Ordering::Greater;
    }
    let bits = |value: f64| {
        if value.is_nan() {
            f64::NAN.to_bits() as i64
        } else {
            value.to_bits() as i64
        }
    };
    bits(left).cmp(&bits(right))
}

/// `CollectionPredicate.codec` fields are optional, while each `count` entry
/// has required `test` then required `count` fields.
pub(super) fn decode_collection_predicate(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    mut decode_element: impl FnMut(
        RegistryElementValueRef<'_>,
        &str,
        &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<CanonicalRegistryCodecValue, RegistryCodecError>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "collection predicate")?;
    let mut output = RegistryCodecRecord::default();
    if let Some(contains) = input.field("contains") {
        let contains_field = child(field, "contains");
        let decoded = context.with_nested(&contains_field, |context| {
            decode_list(
                contains,
                &contains_field,
                context,
                |value, item_field, context| decode_element(value, item_field, context),
            )
        })?;
        output.push("contains", decoded);
    }
    if let Some(counts) = input.field("count") {
        let count_field = child(field, "count");
        let decoded = context.with_nested(&count_field, |context| {
            decode_list(
                counts,
                &count_field,
                context,
                |value, item_field, context| {
                    context.charge_record(value, item_field, "collection count predicate")?;
                    let test_field = child(item_field, "test");
                    let test = required_field(value, "test", item_field, context)?;
                    let test = context.with_nested(&test_field, |context| {
                        decode_element(test, &test_field, context)
                    })?;
                    let bounds_field = child(item_field, "count");
                    let bounds = required_field(value, "count", item_field, context)?;
                    let bounds = context.with_nested(&bounds_field, |context| {
                        decode_int_bounds(bounds, &bounds_field, context)
                    })?;
                    let mut entry = RegistryCodecRecord::default();
                    entry.push("test", test);
                    entry.push("count", bounds.value);
                    Ok(entry.finish())
                },
            )
        })?;
        output.push("count", decoded);
    }
    if let Some(size) = input.field("size") {
        let size_field = child(field, "size");
        let decoded = context.with_nested(&size_field, |context| {
            decode_int_bounds(size, &size_field, context)
        })?;
        output.push("size", decoded.value);
    }
    Ok(output.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry_codec::{
        decode_direct_registry_json, RegistryElementData, RegistryElementSource,
    };
    use bbb_protocol::NbtSummaryEntry;

    const OWNER: &str = "minecraft:data_component_type";
    const CODEC: &str = "DataComponentPatch.CODEC";

    fn context(source: RegistryElementSource) -> RegistryCodecDecodeContext<'static> {
        RegistryCodecDecodeContext::new(OWNER, CODEC, source)
    }

    fn server(json: &[u8]) -> RegistryElementData {
        decode_direct_registry_json(json).unwrap()
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

    #[test]
    fn collection_keeps_optional_empty_size_and_required_test_count_order() {
        let data = server(br#"{"count":[{"count":2,"test":"x"}],"size":{}}"#);
        let decoded = decode_collection_predicate(
            data.root(),
            "$",
            &mut context(data.source),
            |input, field, context| decode_string(input, field, context, "test value"),
        )
        .unwrap();
        assert_eq!(
            decoded.network_nbt_unchecked(),
            &compound(vec![
                entry(
                    "count",
                    list(vec![compound(vec![
                        entry("test", string("x")),
                        entry("count", NbtSummaryValue::Int(2)),
                    ])]),
                ),
                entry("size", compound(Vec::new())),
            ])
        );

        let missing = server(br#"{"count":[{"count":2}]}"#);
        assert!(decode_collection_predicate(
            missing.root(),
            "$",
            &mut context(missing.source),
            |input, field, context| decode_string(input, field, context, "test value"),
        )
        .is_err());
    }
}
