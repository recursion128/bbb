//! Exact source-aware owner for Vanilla 26.1 `BlockState.CODEC`.
//!
//! `StateHolder.codec` selects the block through `Name`, decodes `Properties`
//! leniently against that block's generated state definition, and writes the
//! canonical properties before `Name` (`StateHolder`).

use std::collections::BTreeMap;

use bbb_facts::{
    block_state_types::{
        vanilla_block_default_state_id, vanilla_block_property_count,
        vanilla_block_property_values, vanilla_block_state_block_type_id,
        vanilla_block_state_id_by_name_and_properties, vanilla_block_state_name,
        vanilla_block_state_properties,
    },
    block_types::vanilla_block_type_for_resource_id,
};
use bbb_protocol::{content_registry::ContentRegistry, NbtSummaryEntry, NbtSummaryValue};

use super::{
    recursive_values::{
        required_field as required, CanonicalRegistryCodecValue, RegistryCodecDecodeContext,
    },
    RegistryCodecError, RegistryElementValueRef, RegistryResourceIdentifier,
};

/// Reads a bounded NBT state using the same name and lenient property owner.
pub fn decode_block_state_id_nbt(
    value: &NbtSummaryValue,
    field: &str,
    limits: crate::PersistentNbtDecodeLimits,
) -> Result<i32, RegistryCodecError> {
    decode_block_state_id_nbt_with_content_registry(
        value,
        field,
        limits,
        ContentRegistry::vanilla(),
    )
}

/// Registry-aware form of [`decode_block_state_id_nbt`].
pub fn decode_block_state_id_nbt_with_content_registry(
    value: &NbtSummaryValue,
    field: &str,
    limits: crate::PersistentNbtDecodeLimits,
    content_registry: &ContentRegistry,
) -> Result<i32, RegistryCodecError> {
    let context = || {
        RegistryCodecDecodeContext::with_limits(
            "minecraft:block",
            "BlockState.CODEC",
            super::RegistryElementSource::NetworkNbt,
            super::recursive_values::RegistryCodecDecodeLimits {
                max_depth: limits.max_depth,
                max_nodes: limits.max_nodes,
                max_collection: limits.max_collection,
            },
        )
        .map(|ctx| ctx.with_content_registry(content_registry))
    };
    let input = RegistryElementValueRef::Nbt(value);
    super::native_outcome::validate_raw_input(input, field, &mut context()?)?;
    decode_block_state_id(input, field, &mut context()?)
}

/// Decodes and canonically encodes `BlockState.CODEC` from any registry source.
pub(crate) fn decode_block_state_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let state_id = decode_block_state_id(input, field, context)?;
    encode_block_state(state_id, field, context)
}

/// Decodes the complete input state, then writes its block's default state.
///
/// This is the source shape selected by `RotatedBlockProvider.CODEC`, whose
/// two `xmap` calls deliberately discard input properties before encoding
pub(crate) fn decode_block_default_state_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let state_id = decode_block_state_id(input, field, context)?;
    let default_id = if let Some(block_id) = vanilla_block_state_block_type_id(state_id) {
        vanilla_block_default_state_id(block_id)
            .ok_or_else(|| context.codec_error(field, "known block has no default state"))?
    } else if let Some((mod_block, _)) = context.content_registry().block_by_state_id(state_id) {
        mod_block.first_state_id
    } else {
        return Err(context.codec_error(field, "unknown generated block state"));
    };
    encode_block_state(default_id, field, context)
}

fn decode_block_state_id(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<i32, RegistryCodecError> {
    context.charge_record(input, field, "block state")?;
    let name_field = format!("{field}.Name");
    let raw = required(input, "Name", field, context)?
        .java_string_ref()
        .ok_or_else(|| context.codec_error(&name_field, "block Name is not a string"))?;
    let identifier = RegistryResourceIdentifier::parse_java_string(raw)
        .ok_or_else(|| context.codec_error(&name_field, "invalid block identifier"))?;
    let Some(block) = vanilla_block_type_for_resource_id(identifier.as_str()) else {
        if let Some(mod_block) = context
            .content_registry()
            .block_by_resource_id(identifier.as_str())
        {
            return Ok(mod_block.first_state_id);
        }
        return Err(context.codec_error(&name_field, format!("unknown block `{raw}`")));
    };
    let block_name = block.resource_id;
    let properties_input = input.field("Properties");
    let default_id = vanilla_block_default_state_id(block.id)
        .ok_or_else(|| context.codec_error(field, "known block has no default state"))?;
    if vanilla_block_property_count(block_name).unwrap_or(0) == 0 {
        return Ok(default_id);
    }
    let Some(source) = properties_input else {
        return Ok(default_id);
    };
    let Some(len) = source.map_len() else {
        return Ok(default_id);
    };
    context.check_collection(&format!("{field}.Properties"), len)?;
    let mut properties: BTreeMap<String, String> = vanilla_block_state_properties(default_id)
        .map(|(name, value)| (name.to_owned(), value.to_owned()))
        .collect();
    for (name, value) in &mut properties {
        if let Some(raw) = source
            .field(name)
            .and_then(RegistryElementValueRef::java_string_ref)
        {
            if vanilla_block_property_values(block_name, name)
                .is_some_and(|values| values.iter().any(|name| raw == *name))
            {
                *value = raw
                    .try_to_string()
                    .expect("known property value is valid UTF-16");
            }
        }
    }
    vanilla_block_state_id_by_name_and_properties(block_name, &properties).ok_or_else(|| {
        context.codec_error(
            field,
            "block state does not resolve to a generated Vanilla state",
        )
    })
}

fn encode_block_state(
    state_id: i32,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    if let Some(block_name) = vanilla_block_state_name(state_id) {
        let mut nbt = Vec::with_capacity(2);
        let property_count = vanilla_block_property_count(block_name).unwrap_or(0);
        if property_count != 0 {
            let properties_field = format!("{field}.Properties");
            let mut entries = Vec::with_capacity(property_count);
            context.charge_node(&properties_field)?;
            for (name, value) in vanilla_block_state_properties(state_id) {
                context.charge_node(&format!("{properties_field}.{name}"))?;
                entries.push(NbtSummaryEntry {
                    name: name.into(),
                    value: NbtSummaryValue::string(value),
                });
            }
            entries.sort_by(|left, right| left.name.units().cmp(right.name.units()));
            nbt.push(NbtSummaryEntry {
                name: "Properties".into(),
                value: NbtSummaryValue::Compound(entries),
            });
        }

        let name_field = format!("{field}.Name");
        context.charge_node(&name_field)?;
        nbt.push(NbtSummaryEntry {
            name: "Name".into(),
            value: NbtSummaryValue::string(block_name),
        });

        return Ok(CanonicalRegistryCodecValue::new(NbtSummaryValue::Compound(
            nbt,
        )));
    }
    let Some((mod_block, _)) = context.content_registry().block_by_state_id(state_id) else {
        return Err(context.codec_error(field, "unknown generated block state"));
    };
    let block_name = mod_block.resource_id.clone();
    let name_field = format!("{field}.Name");
    context.charge_node(&name_field)?;
    Ok(CanonicalRegistryCodecValue::new(NbtSummaryValue::Compound(
        vec![NbtSummaryEntry {
            name: "Name".into(),
            value: NbtSummaryValue::string(&block_name),
        }],
    )))
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod native_read_tests;
