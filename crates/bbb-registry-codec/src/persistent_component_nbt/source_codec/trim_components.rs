//! Source-aware armor-trim component owners.
//!
//! `ArmorTrim` contains two `RegistryFileCodec` holders, while the
//! `provides_trim_material` component is itself one such holder. Named values
//! stay unresolved until the frozen candidate validates them; inline material
//! and pattern definitions share the recursive Component budget

use crate::component_source_codec::decode_component_source;
use std::collections::HashSet;

use crate::registry_codec::{
    recursive_values::{required_field, CanonicalRegistryCodecValue, RegistryCodecDecodeContext},
    RegistryCodecError, RegistryElementValueRef, RegistryResourceIdentifier,
};
use bbb_protocol::{
    nbt_summary::entry, packets::is_valid_resource_path, NbtSummaryEntry, NbtSummaryValue,
};

use super::decode_identifier;

pub(crate) fn decode_trim_material_direct(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "trim material")?;

    let asset_field = child(field, "asset_name");
    let asset = required_field(input, "asset_name", field, context)?;
    let asset = decode_resource_path(asset, &asset_field, context)?;
    let overrides = if let Some(overrides) = input.field("override_armor_assets") {
        let overrides_field = child(field, "override_armor_assets");
        context.with_nested(&overrides_field, |context| {
            decode_asset_overrides(overrides, &overrides_field, context)
        })?
    } else {
        Vec::new()
    };

    let description_field = child(field, "description");
    let description = required_field(input, "description", field, context)?;
    let description = context.with_nested(&description_field, |context| {
        decode_component_source(description, &description_field, context)
    })?;
    let (description, references) = description.into_parts();
    let mut fields = vec![entry("asset_name", NbtSummaryValue::String(asset.into()))];
    if !overrides.is_empty() {
        fields.push(entry(
            "override_armor_assets",
            NbtSummaryValue::Compound(overrides),
        ));
    }
    fields.push(entry("description", description));
    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::Compound(fields),
        references,
    ))
}

pub(crate) fn decode_trim_pattern_direct(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "trim pattern")?;

    let asset_field = child(field, "asset_id");
    let asset = decode_identifier(
        required_field(input, "asset_id", field, context)?,
        &asset_field,
        context,
    )?;
    let description_field = child(field, "description");
    let description = required_field(input, "description", field, context)?;
    let description = context.with_nested(&description_field, |context| {
        decode_component_source(description, &description_field, context)
    })?;

    // `Codec.BOOL.fieldOf("decal").orElse(false)` discards a rejected value
    // and always writes the selected boolean.
    let decal = match input.field("decal") {
        Some(value) => {
            let decal_field = child(field, "decal");
            context
                .speculate_codec_alternative(|context| {
                    context.charge_node(&decal_field)?;
                    value
                        .as_bool()
                        .ok_or_else(|| context.codec_error(&decal_field, "decal is not a boolean"))
                })?
                .unwrap_or(false)
        }
        None => false,
    };

    let (description, references) = description.into_parts();
    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::Compound(vec![
            entry("asset_id", NbtSummaryValue::String(asset.as_str().into())),
            entry("description", description),
            entry("decal", NbtSummaryValue::Byte(i8::from(decal))),
        ]),
        references,
    ))
}

fn decode_asset_overrides(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<Vec<NbtSummaryEntry>, RegistryCodecError> {
    let len = context.charge_record(input, field, "override_armor_assets")?;

    let visible = input.visible_map_entries(len);

    let mut normalized = HashSet::with_capacity(visible.len());
    let mut output = Vec::with_capacity(visible.len());
    for (raw, value) in visible {
        let entry_field = format!("{field}.{raw}");
        let identifier = RegistryResourceIdentifier::parse_java_string(raw).ok_or_else(|| {
            context.codec_error(
                &entry_field,
                format!("invalid equipment asset identifier {raw}"),
            )
        })?;
        if !normalized.insert(identifier.as_str().to_string()) {
            return Err(context.codec_error(
                &entry_field,
                format!(
                    "duplicate normalized equipment asset {}",
                    identifier.as_str()
                ),
            ));
        }
        let suffix = decode_resource_path(value, &entry_field, context)?;
        output.push(entry(
            identifier.as_str(),
            NbtSummaryValue::String(suffix.into()),
        ));
    }
    Ok(output)
}

fn decode_resource_path(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<String, RegistryCodecError> {
    context.charge_node(field)?;
    let value = input
        .utf8_string()
        .ok_or_else(|| context.codec_error(field, "expected a resource path string"))?;
    if !is_valid_resource_path(&value) {
        return Err(context.codec_error(field, format!("invalid resource path {value}")));
    }
    Ok(value)
}

fn child(field: &str, name: &str) -> String {
    format!("{field}.{name}")
}
