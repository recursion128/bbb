use std::collections::{HashMap, HashSet};

use crate::registry_codec::{
    recursive_values::{
        CanonicalRegistryCodecValue, OwnedRegistryCodecReference, RegistryCodecDecodeContext,
    },
    RegistryCodecError, RegistryElementValueRef,
};
use bbb_facts::data_component_types;
use bbb_protocol::{nbt_summary::entry, NbtSummaryValue};

use super::{decode_component_value_source, normalize_component_identifier};

/// Decodes Java 26.1 `DataComponentPatch.CODEC` without consulting registry
/// membership or applying item-prototype sanitation. The dispatched map and
/// raw patch projection are defined at `DataComponentPatch.java:21-57`; key
/// parsing and the empty-map removal value sit at
/// `DataComponentPatch.java:289-317`.
pub(crate) fn decode_data_component_patch_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let len = context.charge_record(input, field, "data component patch")?;

    let mut decoded_keys = HashSet::with_capacity(len);
    let mut positions = HashMap::with_capacity(len);
    let mut output = Vec::with_capacity(len);
    let mut references: Vec<OwnedRegistryCodecReference> = Vec::new();
    for index in 0..len {
        let (raw_key, value) = input
            .map_entry_java_string_ref(index)
            .expect("map length and indexed entries stay consistent");
        let raw_key = String::from_utf16(raw_key.units()).map_err(|_| {
            context.codec_error(field, "data component key is not a resource identifier")
        })?;
        let entry_field = format!("{field}.{raw_key}");
        let (removed, descriptor) = decode_patch_key(&raw_key, &entry_field, context)?;
        let identifier = descriptor.resource_id;
        if !decoded_keys.insert((descriptor.type_id, removed)) {
            return Err(context.codec_error(
                &entry_field,
                format!("duplicate data component patch key {identifier}"),
            ));
        }

        let canonical = if removed {
            decode_removal(value, &entry_field, context)?;
            NbtSummaryValue::Compound(Vec::new())
        } else {
            let decoded = context.with_nested(&entry_field, |context| {
                decode_component_value_source(descriptor, value, &entry_field, context)
            })?;
            let (network_nbt, nested_references) = decoded.into_parts();
            references.extend(nested_references);
            network_nbt
        };
        let key = if removed {
            format!("!{}", descriptor.resource_id)
        } else {
            descriptor.resource_id.to_string()
        };
        if let Some(&position) = positions.get(&descriptor.type_id) {
            output[position] = entry(&key, canonical);
        } else {
            positions.insert(descriptor.type_id, output.len());
            output.push(entry(&key, canonical));
        }
    }

    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::Compound(output),
        references,
    ))
}

fn decode_patch_key(
    raw_key: &str,
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> Result<
    (
        bool,
        &'static data_component_types::DataComponentTypeCatalogEntry,
    ),
    RegistryCodecError,
> {
    let (removed, raw_identifier) = raw_key
        .strip_prefix('!')
        .map_or((false, raw_key), |identifier| (true, identifier));
    let identifier = normalize_component_identifier(raw_identifier, field, context)?;
    let descriptor = data_component_types::vanilla_data_component_type_for_resource_id(&identifier)
        .ok_or_else(|| {
            context.codec_error(field, format!("unknown data component {identifier}"))
        })?;
    if !descriptor.persistent {
        return Err(context.codec_error(
            field,
            format!("{} is a transient data component", descriptor.resource_id),
        ));
    }
    Ok((removed, descriptor))
}

fn decode_removal(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    context.charge_node(field)?;
    if !input.is_compound() {
        return Err(context.codec_error(field, "removed data component payload is not a map"));
    }
    Ok(())
}
