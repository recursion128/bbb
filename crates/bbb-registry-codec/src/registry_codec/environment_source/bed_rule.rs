//! Exact `BedRule.CODEC` value owner.

use crate::component_source_codec::decode_component_source;
use crate::registry_codec::{
    recursive_values::{required_field, CanonicalRegistryCodecValue, RegistryCodecDecodeContext},
    RegistryCodecError, RegistryElementValueRef,
};
use bbb_protocol::{nbt_summary::entry, NbtSummaryValue};

pub(super) fn decode_bed_rule_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "bed rule")?;

    let can_sleep = decode_rule(
        required_field(input, "can_sleep", field, context)?,
        "can_sleep",
        context,
    )?;
    let can_set_spawn = decode_rule(
        required_field(input, "can_set_spawn", field, context)?,
        "can_set_spawn",
        context,
    )?;

    let explodes = input
        .field("explodes")
        .map(|value| context.decode_bool(value, "explodes"))
        .transpose()?
        .unwrap_or(false);

    let mut fields = vec![
        entry("can_sleep", NbtSummaryValue::String(can_sleep.into())),
        entry(
            "can_set_spawn",
            NbtSummaryValue::String(can_set_spawn.into()),
        ),
    ];
    if explodes {
        fields.push(entry("explodes", NbtSummaryValue::Byte(1)));
    }

    let references = if let Some(error_message) = input.field("error_message") {
        let message = context.with_nested("error_message", |context| {
            decode_component_source(error_message, "error_message", context)
        })?;
        let (message, references) = message.into_parts();
        fields.push(entry("error_message", message));
        references
    } else {
        Vec::new()
    };
    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::Compound(fields),
        references,
    ))
}

fn decode_rule<'a>(
    input: RegistryElementValueRef<'a>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<&'static str, RegistryCodecError> {
    context.charge_node(field)?;
    let value = input
        .java_string_ref()
        .ok_or_else(|| context.codec_error(field, "bed rule value is not a string"))?;
    if value == "always" {
        Ok("always")
    } else if value == "when_dark" {
        Ok("when_dark")
    } else if value == "never" {
        Ok("never")
    } else {
        Err(context.codec_error(field, format!("unknown bed rule `{value}`")))
    }
}
