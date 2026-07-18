use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use serde::{Deserialize, Serialize};

#[cfg(test)]
use crate::RegistrySet;

use super::super::super::{decode_component, DialogDecodeContext, DialogTextState};
use super::{
    component_f32_bits, find_entry,
    holder_set::decode_registry_holder_reference,
    sound_event::{decode_sound_event_holder, DialogSoundEventHolderState},
    validate_component_float_exclusive_min,
};

const INSTRUMENT_REGISTRY: &str = "minecraft:instrument";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "holder_kind", rename_all = "snake_case")]
pub enum DialogInstrumentHolderState {
    Named {
        registry_id: i32,
        resource_id: String,
    },
    Direct {
        instrument: DialogInstrumentState,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogInstrumentState {
    pub sound_event: DialogSoundEventHolderState,
    pub use_duration_bits: u32,
    pub range_bits: u32,
    pub description: DialogTextState,
}

pub(super) fn decode_instrument_holder(
    value: &NbtSummaryValue,
    context: DialogDecodeContext<'_>,
    what: &str,
) -> Result<DialogInstrumentHolderState, String> {
    match value {
        NbtSummaryValue::String(_) => {
            let holder = decode_registry_holder_reference(
                value,
                INSTRUMENT_REGISTRY,
                context.registry_access,
                what,
            )?;
            Ok(DialogInstrumentHolderState::Named {
                registry_id: holder.holder_id,
                resource_id: holder.resource_id,
            })
        }
        NbtSummaryValue::Compound(entries) => Ok(DialogInstrumentHolderState::Direct {
            instrument: decode_direct_instrument(entries, context, what)?,
        }),
        _ => Err(format!(
            "{what} holder is neither a resource id nor a direct instrument"
        )),
    }
}

fn decode_direct_instrument(
    entries: &[NbtSummaryEntry],
    context: DialogDecodeContext<'_>,
    what: &str,
) -> Result<DialogInstrumentState, String> {
    let sound_event = required_entry(entries, "sound_event", what)?;
    let use_duration_bits = required_positive_float(entries, "use_duration", what)?;
    let range_bits = required_positive_float(entries, "range", what)?;
    Ok(DialogInstrumentState {
        sound_event: decode_sound_event_holder(
            sound_event,
            &format!("{what} direct holder field sound_event"),
        )?,
        use_duration_bits,
        range_bits,
        description: decode_component(
            required_entry(entries, "description", what)?,
            &format!("{what} direct holder field description"),
            context,
        )?,
    })
}

fn required_positive_float(
    entries: &[NbtSummaryEntry],
    field: &str,
    what: &str,
) -> Result<u32, String> {
    let bits = component_f32_bits(
        required_entry(entries, field, what)?,
        &format!("{what} direct holder field {field}"),
    )?;
    validate_component_float_exclusive_min(
        bits,
        0.0,
        f32::MAX,
        &format!("{what} direct holder field {field}"),
    )?;
    Ok(bits)
}

fn required_entry<'a>(
    entries: &'a [NbtSummaryEntry],
    field: &str,
    what: &str,
) -> Result<&'a NbtSummaryValue, String> {
    find_entry(entries, field)
        .ok_or_else(|| format!("{what} direct holder is missing field {field}"))
}

#[cfg(test)]
mod tests;
