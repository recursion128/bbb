//! Source-aware holder primitives shared by persistent component codecs.
//!
//! Named elements and tags remain placeholders until the enclosing frozen
//! registry generation validates them.  Decoding never dereferences a holder
//! target, which preserves Vanilla's legal self- and mutual-reference load
//! order.

use std::collections::{HashMap, HashSet};

use crate::registry_codec::{
    recursive_values::{
        CanonicalRegistryCodecValue, OwnedRegistryCodecReference, RegistryCodecDecodeContext,
    },
    sound_variants::{
        decode_sound_event_holder as decode_shared_sound_event_holder, encode_sound_event_holder,
        SoundEventHolder,
    },
    RegistryCodecError, RegistryElementValueRef, RegistryResourceIdentifier,
};
use bbb_protocol::{nbt_summary::entry, JavaString, NbtSummaryValue};

const SOUND_EVENT_REGISTRY: &str = "minecraft:sound_event";

pub(crate) fn decode_fixed_holder_reference(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    target_registry: &'static str,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let identifier = decode_holder_identifier(input, field, context)?;
    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::String(identifier.as_str().into()),
        vec![OwnedRegistryCodecReference::element(
            target_registry,
            identifier,
        )],
    ))
}

/// Decodes `RegistryCodecs.homogeneousList(registry, alwaysUseList)` when the
/// element codec is `RegistryFixedCodec`.
///
/// A named tag keeps its compact `#id` form. Direct sets accept a scalar when
/// `always_use_list` is false and otherwise use a list; encoding compacts only
/// a one-element direct set (`HolderSetCodec`).
pub(super) fn decode_homogeneous_fixed_holder_set(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    target_registry: &'static str,
    always_use_list: bool,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    if let Some(raw) = input
        .java_string_ref()
        .and_then(|raw| raw.units().strip_prefix(&[b'#' as u16]))
        .and_then(|units| String::from_utf16(units).ok())
    {
        context.charge_node(field)?;
        let identifier = RegistryResourceIdentifier::parse_owned(raw)
            .ok_or_else(|| context.codec_error(field, "invalid tag identifier"))?;
        return Ok(CanonicalRegistryCodecValue::from_parts(
            NbtSummaryValue::String(format!("#{}", identifier.as_str()).into()),
            vec![OwnedRegistryCodecReference::tag(
                target_registry,
                identifier,
            )],
        ));
    }

    let Some(items) = input.list_elements() else {
        if always_use_list {
            return Err(context.codec_error(field, "expected a holder list or #tag"));
        }
        return decode_fixed_holder_reference(input, field, context, target_registry);
    };

    context.charge_node(field)?;
    let len = items.len();
    context.check_collection(field, len)?;
    let mut values = Vec::with_capacity(len);
    let mut references = Vec::with_capacity(len);
    for (index, item) in items.enumerate() {
        let item_field = format!("{field}[{index}]");
        let identifier = context.with_nested(&item_field, |context| {
            decode_holder_identifier(item, &item_field, context)
        })?;
        values.push(NbtSummaryValue::String(identifier.as_str().into()));
        references.push(OwnedRegistryCodecReference::element(
            target_registry,
            identifier,
        ));
    }
    let network_nbt = if !always_use_list && len == 1 {
        values.pop().expect("one holder")
    } else {
        NbtSummaryValue::List(values)
    };
    Ok(CanonicalRegistryCodecValue::from_parts(
        network_nbt,
        references,
    ))
}

/// Decodes an unbounded map whose keys use `RegistryFixedCodec` and whose
/// values are owned by the caller.
///
/// Holder keys are normalized and retained as unresolved references before
/// their corresponding value references, matching the key-then-value decode
/// order of DFU's map codec. Exact duplicate source keys are last-wins;
/// distinct spellings that normalize to the same key remain an error.
pub(crate) fn decode_fixed_holder_keyed_map(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    target_registry: &'static str,
    mut decode_value: impl FnMut(
        RegistryElementValueRef<'_>,
        &str,
        &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<CanonicalRegistryCodecValue, RegistryCodecError>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let visible = visible_holder_map_entries(input, field, context)?;

    let mut normalized_keys = HashSet::with_capacity(visible.len());
    let mut output = Vec::with_capacity(visible.len());
    let mut references = Vec::new();
    for (raw_key, value) in visible {
        let entry_field = format!("{field}.{}", raw_key.to_jdk_string());
        let identifier =
            RegistryResourceIdentifier::parse_java_string(raw_key).ok_or_else(|| {
                context.codec_error(&entry_field, format!("invalid holder identifier {raw_key}"))
            })?;
        if !normalized_keys.insert(identifier.as_str().to_string()) {
            return Err(context.codec_error(
                &entry_field,
                format!("duplicate normalized holder {}", identifier.as_str()),
            ));
        }

        let decoded = context.with_nested(&entry_field, |context| {
            decode_value(value, &entry_field, context)
        })?;
        let (network_nbt, nested_references) = decoded.into_parts();
        output.push(entry(identifier.as_str(), network_nbt));
        references.push(OwnedRegistryCodecReference::element(
            target_registry,
            identifier,
        ));
        references.extend(nested_references);
    }

    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::Compound(output),
        references,
    ))
}

fn visible_holder_map_entries<'a>(
    input: RegistryElementValueRef<'a>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<Vec<(&'a JavaString, RegistryElementValueRef<'a>)>, RegistryCodecError> {
    let len = context.charge_record(input, field, "holder-keyed map")?;
    let mut positions: HashMap<&JavaString, usize> = HashMap::with_capacity(len);
    let mut visible: Vec<(&JavaString, RegistryElementValueRef<'_>)> = Vec::with_capacity(len);
    for index in 0..len {
        let (raw_key, value) = input
            .map_entry_java_string_ref(index)
            .expect("holder-map length and indexed entries stay consistent");
        if let Some(&position) = positions.get(raw_key) {
            visible[position].1 = value;
        } else {
            positions.insert(raw_key, visible.len());
            visible.push((raw_key, value));
        }
    }
    Ok(visible)
}

/// Reuses the single exact `SoundEvent.CODEC` implementation owned by the
/// synchronized sound-variant codecs. Named values retain an element
/// placeholder; inline values are emitted directly and never consult the
/// sound-event registry.
pub(crate) fn decode_sound_event_file_holder(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    if input.is_compound() {
        let len = input.map_len().expect("compound input has a map length");
        context.check_collection(field, len)?;
        if input.field("sound_id").is_some() {
            context.charge_node(&format!("{field}.sound_id"))?;
        }
        if input
            .field("range")
            .is_some_and(|range| range.codec_f32().is_some())
        {
            context.charge_node(&format!("{field}.range"))?;
        }
    }
    let holder = decode_shared_sound_event_holder(
        input,
        context.owner_registry(),
        context.owner_codec(),
        field,
    )?;
    Ok(encode_sound_holder(holder))
}

fn encode_sound_holder(holder: SoundEventHolder) -> CanonicalRegistryCodecValue {
    let mut decoded = CanonicalRegistryCodecValue::new(encode_sound_event_holder(&holder));
    if let SoundEventHolder::Named(identifier) = holder {
        decoded.push_reference(OwnedRegistryCodecReference::element(
            SOUND_EVENT_REGISTRY,
            identifier,
        ));
    }
    decoded
}

fn decode_holder_identifier(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<RegistryResourceIdentifier, RegistryCodecError> {
    context.charge_node(field)?;
    let raw = input
        .java_string_ref()
        .ok_or_else(|| context.codec_error(field, "expected a named holder identifier"))?;
    RegistryResourceIdentifier::parse_java_string(raw)
        .ok_or_else(|| context.codec_error(field, format!("invalid holder identifier {raw}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::registry_codec::{
        decode_direct_registry_json, validate_registry_codec_references, RegistryElementSource,
        RegistryReferenceLookup,
    };

    const OWNER: &str = "minecraft:dialog";
    const CODEC: &str = "Dialog.DIRECT_CODEC";

    struct Lookup;

    impl RegistryReferenceLookup for Lookup {
        fn contains_element(&self, registry: &str, resource_id: &str) -> bool {
            matches!(
                (registry, resource_id),
                ("minecraft:item", "minecraft:stone")
                    | ("minecraft:item", "minecraft:dirt")
                    | ("minecraft:enchantment", "minecraft:sharpness")
                    | ("minecraft:sound_event", "minecraft:item.break")
            )
        }

        fn contains_tag(&self, registry: &str, resource_id: &str) -> bool {
            (registry, resource_id) == ("minecraft:item", "minecraft:logs")
        }
    }

    fn context() -> RegistryCodecDecodeContext<'static> {
        RegistryCodecDecodeContext::new(OWNER, CODEC, RegistryElementSource::ServerDatapackJson)
    }

    #[test]
    fn fixed_holder_sets_preserve_tag_compact_and_direct_list_forms() {
        for (json, expected) in [
            (
                r##""#logs""##,
                NbtSummaryValue::String("#minecraft:logs".into()),
            ),
            (
                r#""stone""#,
                NbtSummaryValue::String("minecraft:stone".into()),
            ),
            (r#"[]"#, NbtSummaryValue::List(Vec::new())),
            (
                r#"["stone","dirt"]"#,
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::String("minecraft:stone".into()),
                    NbtSummaryValue::String("minecraft:dirt".into()),
                ]),
            ),
        ] {
            let data = decode_direct_registry_json(json.as_bytes()).unwrap();
            let decoded = decode_homogeneous_fixed_holder_set(
                data.root(),
                "$",
                &mut context(),
                "minecraft:item",
                false,
            )
            .unwrap();
            assert_eq!(decoded.network_nbt_unchecked(), &expected);
            validate_registry_codec_references(OWNER, CODEC, decoded.references(), &Lookup)
                .unwrap();
        }
    }

    #[test]
    fn holder_keyed_map_normalizes_keys_and_keeps_key_then_value_references() {
        let data = decode_direct_registry_json(
            br#"{"speed":{"item":"stone"},"slowness":{"item":"dirt"}}"#,
        )
        .unwrap();
        let decoded = decode_fixed_holder_keyed_map(
            data.root(),
            "$",
            &mut context(),
            "minecraft:mob_effect",
            |value, field, context| {
                let item = value
                    .field("item")
                    .ok_or_else(|| context.codec_error(field, "item is required"))?;
                decode_fixed_holder_reference(item, field, context, "minecraft:item")
            },
        )
        .unwrap();
        assert_eq!(
            decoded.network_nbt_unchecked(),
            &NbtSummaryValue::Compound(vec![
                entry(
                    "minecraft:speed",
                    NbtSummaryValue::String("minecraft:stone".into()),
                ),
                entry(
                    "minecraft:slowness",
                    NbtSummaryValue::String("minecraft:dirt".into()),
                ),
            ])
        );
        let references = decoded.references().collect::<Vec<_>>();
        assert_eq!(references.len(), 4);
        assert_eq!(references[0].target_registry, "minecraft:mob_effect");
        assert_eq!(references[0].resource_id.as_str(), "minecraft:speed");
        assert_eq!(references[1].target_registry, "minecraft:item");
        assert_eq!(references[1].resource_id.as_str(), "minecraft:stone");
        assert_eq!(references[2].target_registry, "minecraft:mob_effect");
        assert_eq!(references[2].resource_id.as_str(), "minecraft:slowness");
        assert_eq!(references[3].target_registry, "minecraft:item");
        assert_eq!(references[3].resource_id.as_str(), "minecraft:dirt");

        let aliases = decode_direct_registry_json(br#"{"speed":{},"minecraft:speed":{}}"#).unwrap();
        assert!(decode_fixed_holder_keyed_map(
            aliases.root(),
            "$",
            &mut context(),
            "minecraft:mob_effect",
            |_value, _field, _context| Ok(CanonicalRegistryCodecValue::new(
                NbtSummaryValue::Compound(Vec::new())
            )),
        )
        .is_err());
    }

    #[test]
    fn sound_event_holder_reuses_named_inline_and_lenient_range_semantics() {
        let named = decode_direct_registry_json(br#""item.break""#).unwrap();
        let decoded = decode_sound_event_file_holder(named.root(), "$", &mut context()).unwrap();
        assert_eq!(
            decoded.network_nbt_unchecked(),
            &NbtSummaryValue::String("minecraft:item.break".into())
        );
        validate_registry_codec_references(OWNER, CODEC, decoded.references(), &Lookup).unwrap();

        let inline =
            decode_direct_registry_json(br#"{"sound_id":"example:custom","range":"ignored"}"#)
                .unwrap();
        let decoded = decode_sound_event_file_holder(inline.root(), "$", &mut context()).unwrap();
        assert_eq!(
            decoded.network_nbt_unchecked(),
            &NbtSummaryValue::Compound(vec![entry(
                "sound_id",
                NbtSummaryValue::String("example:custom".into()),
            )])
        );
        assert_eq!(decoded.references().count(), 0);
    }
}
