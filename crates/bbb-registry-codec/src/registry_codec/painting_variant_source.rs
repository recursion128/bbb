//! Exact synchronized source owner for Vanilla 26.1
//! `PaintingVariant.DIRECT_CODEC`.
//!
//! The record writes `width`, `height`, normalized `asset_id`, then optional
//! title and author Components. Width and
//! height use `Codec.INT` narrowing before the inclusive `1..=16` check.

use crate::component_source_codec::decode_component_source;

use crate::registry_codec::{
    recursive_values::{required_field, CanonicalRegistryCodecValue, RegistryCodecDecodeContext},
    synchronized_registry_element_codec, RegistryCodecError, RegistryElementData,
    RegistryElementValueRef, RegistryResourceIdentifier,
};
use bbb_protocol::{nbt_summary::entry, NbtSummaryValue};

const PAINTING_VARIANT_REGISTRY: &str = "minecraft:painting_variant";

pub(crate) fn decode_painting_variant_element_with_content_registry(
    data: &RegistryElementData,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let codec = synchronized_registry_element_codec(PAINTING_VARIANT_REGISTRY, data.source)?;
    let mut context =
        RegistryCodecDecodeContext::new(PAINTING_VARIANT_REGISTRY, codec, data.source)
            .with_content_registry(content_registry);
    decode_painting_variant(data.root(), "<root>", &mut context)
}

#[cfg(test)]
pub(crate) fn decode_painting_variant_element(
    data: &RegistryElementData,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    decode_painting_variant_element_with_content_registry(
        data,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

fn decode_painting_variant(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "painting variant")?;

    let width = context.decode_i32_range(
        required_field(input, "width", field, context)?,
        "width",
        1,
        16,
    )?;
    let height = context.decode_i32_range(
        required_field(input, "height", field, context)?,
        "height",
        1,
        16,
    )?;
    context.charge_node("asset_id")?;
    let raw_asset_id = required_field(input, "asset_id", field, context)?
        .java_string_ref()
        .ok_or_else(|| context.codec_error("asset_id", "expected an Identifier string"))?;
    let asset_id = RegistryResourceIdentifier::parse_java_string(raw_asset_id)
        .map(RegistryResourceIdentifier::into_string)
        .ok_or_else(|| context.codec_error("asset_id", "invalid painting asset identifier"))?;

    let mut fields = vec![
        entry("width", NbtSummaryValue::Int(width)),
        entry("height", NbtSummaryValue::Int(height)),
        entry("asset_id", NbtSummaryValue::String(asset_id.into())),
    ];
    let mut references = Vec::new();
    for name in ["title", "author"] {
        let Some(component_input) = input.field(name) else {
            continue;
        };
        let component = context.with_nested(name, |context| {
            decode_component_source(component_input, name, context)
        })?;
        let (component_nbt, component_references) = component.into_parts();
        fields.push(entry(name, component_nbt));
        references.extend(component_references);
    }

    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::Compound(fields),
        references,
    ))
}

#[cfg(test)]
mod tests;
