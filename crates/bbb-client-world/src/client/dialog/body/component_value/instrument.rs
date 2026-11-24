use bbb_protocol::packets::InstrumentComponentSummary;
use serde::{Deserialize, Serialize};

use super::super::super::{decode_component, DialogDecodeContext, DialogTextState};
use super::sound_event::{project_sound_event_holder, DialogSoundEventHolderState};

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

pub(super) fn project_instrument_holder(
    instrument: &InstrumentComponentSummary,
    context: DialogDecodeContext<'_>,
) -> Result<DialogInstrumentHolderState, String> {
    if let Some(id) = instrument.registry_id {
        let entry = super::holder_set::project_registry_holder(
            id,
            INSTRUMENT_REGISTRY,
            context.registry_access.unwrap(),
        )?;
        return Ok(DialogInstrumentHolderState::Named {
            registry_id: entry.holder_id,
            resource_id: entry.resource_id,
        });
    }
    let instrument = instrument.direct.as_deref().unwrap();
    let context = DialogDecodeContext {
        prepare_item_summary: false,
        ..context
    };
    Ok(DialogInstrumentHolderState::Direct {
        instrument: DialogInstrumentState {
            sound_event: project_sound_event_holder(
                &instrument.sound_event,
                context.registry_access,
            )?,
            use_duration_bits: instrument.use_duration_bits,
            range_bits: instrument.range_bits,
            description: decode_component(
                instrument.description_nbt.as_deref().unwrap(),
                "instrument description",
                context,
            )?,
        },
    })
}

#[cfg(test)]
mod tests;
