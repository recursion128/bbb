//! Shared source-aware DynamicOps tree projection to canonical NbtOps.
//!
//! NBT input retains its exact tag family while maps project their visible
//! values. JSON input
//! retains the numeric lexeme so the JsonOps number narrowing does not pass
//! through f64 before choosing the canonical NBT tag.

use crate::registry_codec::{
    element_value::collect_visible_map_entries,
    recursive_values::{CanonicalRegistryCodecValue, RegistryCodecDecodeContext},
    RegistryCodecError, RegistryElementValueRef,
};
use bbb_protocol::{
    canonical_snbt_java_string,
    nbt_summary::{
        canonical_snbt_key_java_string, entry, json_number_to_nbtops, JsonOpsNumberError,
    },
    parse_snbt_compound_utf16, JavaString, NbtSummaryEntry, NbtSummaryValue,
};

/// Decodes `TagParser.LENIENT_CODEC` and writes its primary flattened form.
///
/// Both alternatives produce a `CompoundTag`, but `Codec.withAlternative`
/// always encodes through `FLATTENED_CODEC`. The canonical NbtOps output is
/// therefore a string tag containing `CompoundTag.toString()`, and the JsonOps
/// writer is that same SNBT value encoded as a JSON string
pub(crate) fn decode_lenient_flattened_compound_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let mut output = Vec::new();
    if let Some(flattened) = input.java_string() {
        // The accepted codec value is one compound. Charge its source string
        // as that root rather than charging the materialized root a second
        // time, while still walking all parsed children against the shared
        // recursive budget.
        context.charge_node(field)?;
        let parsed = parse_snbt_compound_utf16(flattened.units())
            .map_err(|error| flattened_compound_error(error, field, context))?;
        encode_lenient_compound_snbt(
            RegistryElementValueRef::Nbt(&parsed),
            field,
            context,
            false,
            &mut output,
        )?;
    } else {
        encode_lenient_compound_snbt(input, field, context, true, &mut output)?;
    }
    Ok(CanonicalRegistryCodecValue::new(
        JavaString::from(output).to_nbt(),
    ))
}

fn push_lenient_tag_snbt(value: &NbtSummaryValue, output: &mut Vec<u16>) {
    output.extend_from_slice(canonical_snbt_java_string(value).units());
}

fn flattened_compound_error(
    error: bbb_protocol::SnbtParseFailure,
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> RegistryCodecError {
    let reason = format!("invalid flattened compound: {error}");
    // The parser reserves absent Vanilla keys for its own depth/collection bounds.
    if error.key.is_none() {
        context.budget_error(field, reason)
    } else {
        context.codec_error(field, reason)
    }
}

fn encode_lenient_compound_snbt(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    charge_root: bool,
    output: &mut Vec<u16>,
) -> Result<(), RegistryCodecError> {
    if charge_root {
        context.charge_node(field)?;
    }
    let mut visible = match input {
        RegistryElementValueRef::Nbt(NbtSummaryValue::Compound(values)) => {
            collect_visible_map_entries(
                values
                    .iter()
                    .map(|entry| (&entry.name, RegistryElementValueRef::Nbt(&entry.value))),
                0,
            )
        }
        RegistryElementValueRef::Json(crate::registry_codec::RegistryJsonValue::Object(values)) => {
            values
                .iter()
                .map(|(name, value)| (name, RegistryElementValueRef::Json(value)))
                .collect()
        }
        _ => {
            return Err(
                context.codec_error(field, "NBT predicate is not a compound or SNBT string")
            );
        }
    };
    context.check_collection(field, visible.len())?;
    // StringTagVisitor copies CompoundTag.entrySet() and sorts by Java String
    // natural order before emitting any key/value pair.
    visible.sort_by(|(left, _), (right, _)| left.units().cmp(right.units()));

    output.push(b'{' as u16);
    for (index, (name, value)) in visible.into_iter().enumerate() {
        if index != 0 {
            output.push(b',' as u16);
        }
        output.extend_from_slice(canonical_snbt_key_java_string(name).units());
        output.push(b':' as u16);
        // A codec diagnostic path is text; the error report owns that bound.
        let child_field = format!("{field}.{}", name.to_display_string());
        context.with_nested(&child_field, |context| {
            encode_lenient_snbt_value(value, &child_field, context, output)
        })?;
    }
    output.push(b'}' as u16);
    Ok(())
}

fn encode_lenient_snbt_value(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    output: &mut Vec<u16>,
) -> Result<(), RegistryCodecError> {
    context.charge_node(field)?;
    match input {
        RegistryElementValueRef::Nbt(value) => match value {
            NbtSummaryValue::Compound(_) => {
                encode_lenient_compound_snbt(input, field, context, false, output)
            }
            NbtSummaryValue::List(values) => {
                context.check_collection(field, values.len())?;
                output.push(b'[' as u16);
                for (index, value) in values.iter().enumerate() {
                    if index != 0 {
                        output.push(b',' as u16);
                    }
                    let child_field = format!("{field}[{index}]");
                    context.with_nested(&child_field, |context| {
                        encode_lenient_snbt_value(
                            RegistryElementValueRef::Nbt(value),
                            &child_field,
                            context,
                            output,
                        )
                    })?;
                }
                output.push(b']' as u16);
                Ok(())
            }
            NbtSummaryValue::ByteArray(values) => {
                context.check_collection(field, values.len())?;
                context.charge_nodes(field, values.len())?;
                push_lenient_tag_snbt(value, output);
                Ok(())
            }
            NbtSummaryValue::IntArray(values) => {
                context.check_collection(field, values.len())?;
                context.charge_nodes(field, values.len())?;
                push_lenient_tag_snbt(value, output);
                Ok(())
            }
            NbtSummaryValue::LongArray(values) => {
                context.check_collection(field, values.len())?;
                context.charge_nodes(field, values.len())?;
                push_lenient_tag_snbt(value, output);
                Ok(())
            }
            _ => {
                push_lenient_tag_snbt(value, output);
                Ok(())
            }
        },
        RegistryElementValueRef::NbtByte(value) => {
            push_lenient_tag_snbt(&NbtSummaryValue::Byte(value), output);
            Ok(())
        }
        RegistryElementValueRef::NbtInt(value) => {
            push_lenient_tag_snbt(&NbtSummaryValue::Int(value), output);
            Ok(())
        }
        RegistryElementValueRef::NbtLong(value) => {
            push_lenient_tag_snbt(&NbtSummaryValue::Long(value), output);
            Ok(())
        }
        RegistryElementValueRef::Json(value) => match value {
            crate::registry_codec::RegistryJsonValue::Null => {
                output.extend_from_slice(&[b'E' as u16, b'N' as u16, b'D' as u16]);
                Ok(())
            }
            crate::registry_codec::RegistryJsonValue::Bool(value) => {
                push_lenient_tag_snbt(&NbtSummaryValue::Byte(i8::from(*value)), output);
                Ok(())
            }
            crate::registry_codec::RegistryJsonValue::Number(value) => {
                let value = json_number_to_nbtops(value)
                    .map_err(|error| json_ops_number_error(error, field, context))?;
                push_lenient_tag_snbt(&value, output);
                Ok(())
            }
            crate::registry_codec::RegistryJsonValue::String(value) => {
                push_lenient_tag_snbt(&NbtSummaryValue::String(value.clone()), output);
                Ok(())
            }
            crate::registry_codec::RegistryJsonValue::Array(values) => {
                context.check_collection(field, values.len())?;
                output.push(b'[' as u16);
                for (index, value) in values.iter().enumerate() {
                    if index != 0 {
                        output.push(b',' as u16);
                    }
                    let child_field = format!("{field}[{index}]");
                    context.with_nested(&child_field, |context| {
                        encode_lenient_snbt_value(
                            RegistryElementValueRef::Json(value),
                            &child_field,
                            context,
                            output,
                        )
                    })?;
                }
                output.push(b']' as u16);
                Ok(())
            }
            crate::registry_codec::RegistryJsonValue::Object(_) => {
                encode_lenient_compound_snbt(input, field, context, false, output)
            }
        },
    }
}

/// Decodes `CustomData.COMPOUND_TAG_CODEC` and always writes its primary
/// `CompoundTag.CODEC` form.
///
/// The source codec accepts either a DynamicOps map or flattened SNBT. The
/// latter materializes a second semantic tree, so the shared safety budget
/// charges both the source string and the parsed compound rather than letting
/// the alternative bypass recursive limits (`CustomData`).
#[cfg(test)]
pub(crate) fn decode_compound_tag_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    Ok(decode_compound_tag_dynamic_source(input, field, context)?
        .into_parts()
        .0)
}

/// Decodes `CustomData.COMPOUND_TAG_CODEC`: a map or flattened SNBT
pub(crate) fn decode_compound_tag_dynamic_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let parsed;
    let input = if let Some(flattened) = input.java_string() {
        context.charge_node(field)?;
        parsed = parse_snbt_compound_utf16(flattened.units())
            .map_err(|error| flattened_compound_error(error, field, context))?;
        RegistryElementValueRef::Nbt(&parsed)
    } else {
        input
    };
    if !input.is_compound() {
        return Err(context.codec_error(field, "custom data is neither a map nor flattened SNBT"));
    }
    let typed_compound = decode_dynamic_value(input, field, context)?;
    Ok(CanonicalRegistryCodecValue::new(typed_compound))
}

/// Decodes `ExtraCodecs.NBT`, which normalizes JSON input through NbtOps.
pub(crate) fn decode_nbtops_dynamic_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let typed_tag = decode_dynamic_value(input, field, context)?;
    Ok(CanonicalRegistryCodecValue::new(typed_tag))
}

#[cfg(test)]
pub(crate) fn decode_nbtops_nbt_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    decode_dynamic_value(input, field, context)
}

fn decode_dynamic_value(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    context.charge_node(field)?;
    match input {
        RegistryElementValueRef::Nbt(value) => match value {
            NbtSummaryValue::ByteArray(values) => {
                context.check_collection(field, values.len())?;
                context.charge_nodes(field, values.len())?;
                Ok(value.clone())
            }
            NbtSummaryValue::IntArray(values) => {
                context.check_collection(field, values.len())?;
                context.charge_nodes(field, values.len())?;
                Ok(value.clone())
            }
            NbtSummaryValue::LongArray(values) => {
                context.check_collection(field, values.len())?;
                context.charge_nodes(field, values.len())?;
                Ok(value.clone())
            }
            NbtSummaryValue::List(values) => {
                context.check_collection(field, values.len())?;
                let mut output = Vec::with_capacity(values.len());
                for (index, value) in values.iter().enumerate() {
                    let child_field = format!("{field}[{index}]");
                    output.push(context.with_nested(&child_field, |context| {
                        decode_dynamic_value(
                            RegistryElementValueRef::Nbt(value),
                            &child_field,
                            context,
                        )
                    })?);
                }
                Ok(NbtSummaryValue::List(output))
            }
            NbtSummaryValue::Compound(values) => {
                let visible = collect_visible_map_entries(
                    values.iter().map(|entry| (&entry.name, &entry.value)),
                    0,
                );
                context.check_collection(field, visible.len())?;
                let mut output = Vec::with_capacity(visible.len());
                for (name, value) in visible {
                    let child_field = format!("{field}.{}", name.to_display_string());
                    let value = context.with_nested(&child_field, |context| {
                        decode_dynamic_value(
                            RegistryElementValueRef::Nbt(value),
                            &child_field,
                            context,
                        )
                    })?;
                    output.push(NbtSummaryEntry {
                        name: name.clone(),
                        value,
                    });
                }
                Ok(NbtSummaryValue::Compound(output))
            }
            scalar => Ok(scalar.clone()),
        },
        RegistryElementValueRef::NbtByte(value) => Ok(NbtSummaryValue::Byte(value)),
        RegistryElementValueRef::NbtInt(value) => Ok(NbtSummaryValue::Int(value)),
        RegistryElementValueRef::NbtLong(value) => Ok(NbtSummaryValue::Long(value)),
        RegistryElementValueRef::Json(value) => match value {
            crate::registry_codec::RegistryJsonValue::Null => {
                // NbtOps and NbtOps allow EndTag in a compound,
                // but the canonical summary and network representation cannot retain it.
                Err(context.coverage_error(
                    field,
                    "JSON null projects to EndTag, whose canonical NBT representation is not implemented",
                ))
            }
            crate::registry_codec::RegistryJsonValue::Bool(value) => {
                Ok(NbtSummaryValue::Byte(i8::from(*value)))
            }
            crate::registry_codec::RegistryJsonValue::Number(value) => json_number_to_nbtops(value)
                .map_err(|error| json_ops_number_error(error, field, context)),
            crate::registry_codec::RegistryJsonValue::String(value) => {
                Ok(NbtSummaryValue::String(value.clone()))
            }
            crate::registry_codec::RegistryJsonValue::Array(values) => {
                context.check_collection(field, values.len())?;
                let mut output = Vec::with_capacity(values.len());
                for (index, value) in values.iter().enumerate() {
                    let child_field = format!("{field}[{index}]");
                    output.push(context.with_nested(&child_field, |context| {
                        decode_dynamic_value(
                            RegistryElementValueRef::Json(value),
                            &child_field,
                            context,
                        )
                    })?);
                }
                Ok(NbtSummaryValue::List(output))
            }
            crate::registry_codec::RegistryJsonValue::Object(values) => {
                context.check_collection(field, values.len())?;
                let mut output = Vec::with_capacity(values.len());
                for (name, value) in values {
                    let child_field = format!("{field}.{}", name.to_display_string());
                    let value = context.with_nested(&child_field, |context| {
                        decode_dynamic_value(
                            RegistryElementValueRef::Json(value),
                            &child_field,
                            context,
                        )
                    })?;
                    output.push(entry(name, value));
                }
                Ok(NbtSummaryValue::Compound(output))
            }
        },
    }
}

fn json_ops_number_error(
    error: JsonOpsNumberError,
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> RegistryCodecError {
    match error {
        JsonOpsNumberError::Invalid(reason) => context.codec_error(field, reason),
        JsonOpsNumberError::GsonLimit(reason) => context.coverage_error(field, reason),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry_codec::{
        decode_direct_registry_json, recursive_values::RegistryCodecDecodeLimits,
        RegistryElementSource,
    };

    const OWNER_REGISTRY: &str = "minecraft:test";
    const OWNER_CODEC: &str = "NbtOps";

    fn context(source: RegistryElementSource) -> RegistryCodecDecodeContext<'static> {
        RegistryCodecDecodeContext::new(OWNER_REGISTRY, OWNER_CODEC, source)
    }

    fn compound_field<'a>(value: &'a NbtSummaryValue, name: &str) -> &'a NbtSummaryValue {
        let NbtSummaryValue::Compound(fields) = value else {
            panic!("expected a compound")
        };
        &fields
            .iter()
            .find(|entry| entry.name == name)
            .unwrap_or_else(|| panic!("missing field {name}"))
            .value
    }

    fn flattened_snbt(value: &CanonicalRegistryCodecValue) -> &bbb_protocol::JavaString {
        let NbtSummaryValue::String(value) = value.network_nbt_unchecked() else {
            panic!("flattened codec must write an NBT string")
        };
        value
    }

    fn assert_flattened_writer(value: &CanonicalRegistryCodecValue, expected_snbt: &str) {
        assert_eq!(flattened_snbt(value), expected_snbt);
    }

    #[test]
    fn lenient_flattened_compound_preserves_primitive_arrays() {
        let source = NbtSummaryValue::Compound(vec![
            entry("longs", NbtSummaryValue::LongArray(vec![5, -6])),
            entry("bytes", NbtSummaryValue::ByteArray(vec![1, -2])),
            entry("ints", NbtSummaryValue::IntArray(vec![3, -4])),
        ]);
        let network = decode_lenient_flattened_compound_source(
            RegistryElementValueRef::Nbt(&source),
            "$",
            &mut context(RegistryElementSource::NetworkNbt),
        )
        .unwrap();
        let expected = "{bytes:[B;1B,-2B],ints:[I;3,-4],longs:[L;5L,-6L]}";
        assert_flattened_writer(&network, expected);

        let flattened = decode_direct_registry_json(
            serde_json::to_string("{longs:[L;5L,-6L],bytes:[B;1b,-2b],ints:[I;3,-4]}")
                .unwrap()
                .as_bytes(),
        )
        .unwrap();
        let flattened = decode_lenient_flattened_compound_source(
            flattened.root(),
            "$",
            &mut context(flattened.source),
        )
        .unwrap();
        assert_flattened_writer(&flattened, expected);
    }

    #[test]
    fn network_nbt_dynamic_writer_covers_nested_values_arrays_and_duplicate_keys() {
        let source = NbtSummaryValue::Compound(vec![
            entry("same", NbtSummaryValue::Int(1234)),
            entry(
                "nested",
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::String("\"\\\u{2028}\u{1f600}".to_string().into()),
                    NbtSummaryValue::Float(1.5_f32.to_bits()),
                    NbtSummaryValue::Double(0.25_f64.to_bits()),
                ]),
            ),
            entry("bytes", NbtSummaryValue::ByteArray(vec![1, -2])),
            entry("ints", NbtSummaryValue::IntArray(vec![3, -4])),
            entry("longs", NbtSummaryValue::LongArray(vec![5, -6])),
            entry("same", NbtSummaryValue::String("last".to_string().into())),
        ]);
        let decoded = decode_nbtops_dynamic_source(
            RegistryElementValueRef::Nbt(&source),
            "$",
            &mut context(RegistryElementSource::NetworkNbt),
        )
        .unwrap();
        assert_eq!(
            compound_field(decoded.network_nbt_unchecked(), "same"),
            &NbtSummaryValue::String("last".to_string().into())
        );
        assert!(matches!(
            compound_field(decoded.network_nbt_unchecked(), "bytes"),
            NbtSummaryValue::ByteArray(_)
        ));
    }

    #[test]
    fn json_source_projects_exact_nbtops_scalar_tags() {
        let data = decode_direct_registry_json(
            br#"{
                "byte": 1,
                "short": 128,
                "int": 32768,
                "long": 2147483648,
                "float": 1.5,
                "double": 0.1,
                "negative_zero": -1e-1000,
                "scaled_integer": 0.0000000000000000000000000000001e31
            }"#,
        )
        .unwrap();
        let decoded =
            decode_nbtops_nbt_source(data.root(), "$", &mut context(data.source)).unwrap();
        let NbtSummaryValue::Compound(fields) = decoded else {
            panic!("JSON object must project to a compound")
        };
        let field = |name: &str| {
            &fields
                .iter()
                .find(|entry| entry.name == name)
                .unwrap()
                .value
        };
        assert_eq!(field("byte"), &NbtSummaryValue::Byte(1));
        assert_eq!(field("short"), &NbtSummaryValue::Short(128));
        assert_eq!(field("int"), &NbtSummaryValue::Int(32_768));
        assert_eq!(field("long"), &NbtSummaryValue::Long(2_147_483_648));
        assert_eq!(field("float"), &NbtSummaryValue::Float(1.5_f32.to_bits()));
        assert_eq!(field("double"), &NbtSummaryValue::Double(0.1_f64.to_bits()));
        assert_eq!(
            field("negative_zero"),
            &NbtSummaryValue::Float(0.0_f32.to_bits())
        );
        assert_eq!(field("scaled_integer"), &NbtSummaryValue::Byte(1));
    }

    #[test]
    fn nbt_source_preserves_types_and_uses_visible_compound_values() {
        let all_types = NbtSummaryValue::Compound(vec![
            entry("byte", NbtSummaryValue::Byte(2)),
            entry("short", NbtSummaryValue::Short(3)),
            entry("long", NbtSummaryValue::Long(4)),
            entry("float", NbtSummaryValue::Float(5.0_f32.to_bits())),
            entry("double", NbtSummaryValue::Double(6.0_f64.to_bits())),
            entry("bytes", NbtSummaryValue::ByteArray(vec![7])),
            entry("ints", NbtSummaryValue::IntArray(vec![8])),
            entry("longs", NbtSummaryValue::LongArray(vec![9])),
        ]);
        let source = NbtSummaryValue::Compound(vec![
            entry("same", NbtSummaryValue::Int(1)),
            entry("all", all_types.clone()),
            entry("same", NbtSummaryValue::String("last".to_string().into())),
        ]);
        let decoded = decode_nbtops_nbt_source(
            RegistryElementValueRef::Nbt(&source),
            "$",
            &mut context(RegistryElementSource::NetworkNbt),
        )
        .unwrap();
        let NbtSummaryValue::Compound(fields) = decoded else {
            panic!("compound input must remain a compound")
        };
        assert_eq!(fields.len(), 2);
        assert_eq!(
            fields[0],
            entry("same", NbtSummaryValue::String("last".to_string().into()))
        );
        assert_eq!(fields[1], entry("all", all_types));
    }

    #[test]
    fn compound_codec_accepts_maps_and_flattened_snbt_with_one_canonical_writer() {
        let direct = decode_direct_registry_json(br#"{"z":1,"nested":{"value":2s}}"#);
        assert!(direct.is_err(), "JSON cannot spell an NBT short suffix");

        let direct = decode_direct_registry_json(br#"{"z":1,"nested":{"value":2}}"#).unwrap();
        let direct =
            decode_compound_tag_source(direct.root(), "$", &mut context(direct.source)).unwrap();

        let flattened =
            decode_direct_registry_json(br#""{z:1b,nested:{value:2s},float:-0.0f,double:-0.0d}""#)
                .unwrap();
        let flattened =
            decode_compound_tag_source(flattened.root(), "$", &mut context(flattened.source))
                .unwrap();
        assert_eq!(
            flattened,
            NbtSummaryValue::Compound(vec![
                entry("z", NbtSummaryValue::Byte(1)),
                entry(
                    "nested",
                    NbtSummaryValue::Compound(vec![entry("value", NbtSummaryValue::Short(2))]),
                ),
                entry("float", NbtSummaryValue::Float(0.0_f32.to_bits())),
                entry("double", NbtSummaryValue::Double(0.0_f64.to_bits())),
            ])
        );
        assert!(matches!(direct, NbtSummaryValue::Compound(_)));

        let scalar = decode_direct_registry_json(br#"7"#).unwrap();
        assert!(
            decode_compound_tag_source(scalar.root(), "$", &mut context(scalar.source),).is_err()
        );

        let materialized = decode_direct_registry_json(br#""{value:1}""#).unwrap();
        let mut exact_budget = RegistryCodecDecodeContext::with_limits(
            OWNER_REGISTRY,
            OWNER_CODEC,
            materialized.source,
            RegistryCodecDecodeLimits {
                max_depth: 2,
                max_nodes: 3,
                max_collection: 2,
            },
        )
        .unwrap();
        decode_compound_tag_source(materialized.root(), "$", &mut exact_budget).unwrap();

        let mut short_budget = RegistryCodecDecodeContext::with_limits(
            OWNER_REGISTRY,
            OWNER_CODEC,
            materialized.source,
            RegistryCodecDecodeLimits {
                max_depth: 2,
                max_nodes: 2,
                max_collection: 2,
            },
        )
        .unwrap();
        assert!(decode_compound_tag_source(materialized.root(), "$", &mut short_budget).is_err());
    }
}
