//! Exact synchronized source owner for Vanilla 26.1 `ChatType.DIRECT_CODEC`.
//!
//! Both decorations are required and retain record order.
//! A decoration writes `translation_key`, `parameters`, and its non-empty
//! standalone `Style` in codec order; the
//! enum accepts only `sender`, `target`, and `content`

use crate::{
    component_source_codec::decode_style_source,
    registry_codec::{
        recursive_values::{
            required_field, CanonicalRegistryCodecValue, RegistryCodecDecodeContext,
        },
        synchronized_registry_element_codec, RegistryCodecError, RegistryElementData,
        RegistryElementValueRef,
    },
};
use bbb_protocol::{nbt_summary::entry, NbtSummaryValue};

const CHAT_TYPE_REGISTRY: &str = "minecraft:chat_type";

pub(crate) fn decode_chat_type_element_with_content_registry(
    data: &RegistryElementData,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let codec = synchronized_registry_element_codec(CHAT_TYPE_REGISTRY, data.source)?;
    let mut context = RegistryCodecDecodeContext::new(CHAT_TYPE_REGISTRY, codec, data.source)
        .with_content_registry(content_registry);
    decode_chat_type(data.root(), "<root>", &mut context)
}

#[cfg(test)]
pub(crate) fn decode_chat_type_element(
    data: &RegistryElementData,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    decode_chat_type_element_with_content_registry(
        data,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

fn decode_chat_type(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "chat type")?;
    let mut fields = Vec::with_capacity(2);
    let mut references = Vec::new();
    for name in ["chat", "narration"] {
        let decoration = context.with_nested(name, |context| {
            decode_decoration(required_field(input, name, field, context)?, name, context)
        })?;
        let (network_nbt, decoration_references) = decoration.into_parts();
        fields.push(entry(name, network_nbt));
        references.extend(decoration_references);
    }

    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::Compound(fields),
        references,
    ))
}

fn decode_decoration(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "chat type decoration")?;

    context.charge_node("translation_key")?;
    let translation_key = required_field(input, "translation_key", field, context)?
        .java_string_ref()
        .ok_or_else(|| context.codec_error("translation_key", "expected Codec.STRING input"))?;

    let parameters = decode_parameters(
        required_field(input, "parameters", field, context)?,
        "parameters",
        context,
    )?;

    let mut fields = vec![
        entry(
            "translation_key",
            NbtSummaryValue::String(translation_key.clone()),
        ),
        entry("parameters", parameters),
    ];
    let mut references = Vec::new();
    if let Some(style) = input.field("style") {
        let style = context.with_nested("style", |context| {
            decode_style_source(style, "style", context)
        })?;
        let (style_nbt, style_references) = style.into_parts();
        references.extend(style_references);
        if !is_empty_compound(&style_nbt) {
            fields.push(entry("style", style_nbt));
        }
    }

    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::Compound(fields),
        references,
    ))
}

fn decode_parameters(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    context.charge_node(field)?;
    let values = input
        .list_elements()
        .ok_or_else(|| context.codec_error(field, "parameters are not a list"))?;
    let len = values.len();
    context.check_collection(field, len)?;
    let mut parameters = Vec::with_capacity(len);
    for value in values {
        context.charge_node(field)?;
        let raw = value
            .java_string_ref()
            .ok_or_else(|| context.codec_error(field, "expected a parameter string"))?;
        if !(raw == "sender" || raw == "target" || raw == "content") {
            return Err(
                context.codec_error(field, format!("unknown chat decoration parameter `{raw}`"))
            );
        }
        parameters.push(NbtSummaryValue::String(raw.clone()));
    }
    Ok(NbtSummaryValue::List(parameters))
}

fn is_empty_compound(value: &NbtSummaryValue) -> bool {
    matches!(value, NbtSummaryValue::Compound(entries) if entries.is_empty())
}

#[cfg(test)]
mod tests;
