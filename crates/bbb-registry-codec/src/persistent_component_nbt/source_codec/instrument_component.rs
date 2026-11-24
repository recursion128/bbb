//! Source-aware `Instrument.CODEC` component owner.
//!
//! The outer holder follows `RegistryFileCodec`: a legal identifier remains
//! an unresolved instrument reference, while every other source shape enters
//! the direct record codec. The inline value keeps the exact field order and
//! recursive Component edge owned by `Instrument`.

use crate::component_source_codec::decode_component_source;
use crate::registry_codec::{
    recursive_values::{required_field, CanonicalRegistryCodecValue, RegistryCodecDecodeContext},
    RegistryCodecError, RegistryElementValueRef,
};
use bbb_protocol::{
    nbt_summary::{canonical_float_tag_bits, entry},
    NbtSummaryValue,
};

use super::{holders::decode_sound_event_file_holder, java_float_compare};

pub(crate) fn decode_inline_instrument(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "inline Instrument record")?;

    let sound_field = child(field, "sound_event");
    let sound = required_field(input, "sound_event", field, context)?;
    let sound = decode_sound_event_file_holder(sound, &sound_field, context)?;

    let use_duration_field = child(field, "use_duration");
    let use_duration = decode_positive_float(
        required_field(input, "use_duration", field, context)?,
        &use_duration_field,
        context,
    )?;

    let range_field = child(field, "range");
    let range = decode_positive_float(
        required_field(input, "range", field, context)?,
        &range_field,
        context,
    )?;

    let description_field = child(field, "description");
    let description = required_field(input, "description", field, context)?;
    let description = context.with_nested(&description_field, |context| {
        decode_component_source(description, &description_field, context)
    })?;
    let (sound, mut references) = sound.into_parts();
    let (description, description_references) = description.into_parts();
    references.extend(description_references);
    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::Compound(vec![
            entry("sound_event", sound),
            entry(
                "use_duration",
                NbtSummaryValue::Float(canonical_float_tag_bits(use_duration)),
            ),
            entry(
                "range",
                NbtSummaryValue::Float(canonical_float_tag_bits(range)),
            ),
            entry("description", description),
        ]),
        references,
    ))
}

fn decode_positive_float(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<f32, RegistryCodecError> {
    context.charge_node(field)?;
    let value = input
        .codec_f32()
        .ok_or_else(|| context.codec_error(field, "expected Codec.FLOAT input"))?;
    // POSITIVE_FLOAT uses boxed Float.compareTo: -0.0 and +0.0 are both
    // rejected by the exclusive lower bound, NaN sorts above Float.MAX_VALUE,
    // and Float.MAX_VALUE itself is accepted (`ExtraCodecs`)
    if !java_float_compare(value, 0.0).is_gt() || java_float_compare(value, f32::MAX).is_gt() {
        return Err(context.codec_error(field, "float must be positive and finite"));
    }
    Ok(value)
}

fn child(parent: &str, name: &str) -> String {
    format!("{parent}.{name}")
}
