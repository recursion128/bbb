use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use serde::{Deserialize, Serialize};

#[cfg(test)]
use crate::RegistrySet;

use super::super::super::{decode_component, DialogDecodeContext, DialogTextState};
use super::{
    component_f32_bits, find_entry,
    sound_event::{decode_sound_event_holder, DialogSoundEventHolderState},
    validate_component_float_exclusive_min,
};
use crate::registries::PersistentHolderOrInline;

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
    let holder = match context.registry_access {
        Some(access) => access
            .persistent_holder_or_inline_from_nbt(INSTRUMENT_REGISTRY, value)
            .map_err(|error| format!("{error} for {what}"))?,
        None if matches!(value, NbtSummaryValue::String(_)) => {
            return Err(format!(
                "{what} holder cannot be decoded without registry context"
            ))
        }
        None => PersistentHolderOrInline::Direct(value),
    };
    match holder {
        PersistentHolderOrInline::Named(entry) => Ok(DialogInstrumentHolderState::Named {
            registry_id: entry.holder_id,
            resource_id: entry.resource_id.to_string(),
        }),
        PersistentHolderOrInline::Direct(NbtSummaryValue::Compound(entries)) => {
            Ok(DialogInstrumentHolderState::Direct {
                instrument: decode_direct_instrument(entries, context, what)?,
            })
        }
        PersistentHolderOrInline::Direct(_) => Err(format!(
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
            context.registry_access,
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
