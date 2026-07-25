use bbb_protocol::{
    data_component_types::VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
    packets::{
        InstrumentSummary, InteractionHand, ItemStackSummary as ProtocolItemStackSummary,
        SoundEventSummary,
    },
};

use crate::{registries::InstrumentSoundEvent, WorldStore};

use super::{LocalInteractionResult, LocalInteractionSuccess, LocalItemUsePrediction};

const SOUND_EVENT_REGISTRY: &str = "minecraft:sound_event";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LocalInstrumentUseState {
    pub sound_event: SoundEventSummary,
    pub use_duration: f32,
    pub range: f32,
}

impl WorldStore {
    pub(super) fn predict_local_instrument_item_use(
        &mut self,
        hand: InteractionHand,
        stack: &ProtocolItemStackSummary,
    ) -> LocalItemUsePrediction {
        if !self
            .item_stack_has_effective_component(stack, VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID)
        {
            return LocalItemUsePrediction::Result(LocalInteractionResult::Fail);
        }
        let Some(instrument) = self.local_instrument_use_state(stack) else {
            return LocalItemUsePrediction::MissingComponentValue(
                VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
            );
        };
        let Some(cooldown_group) = self.local_item_stack_cooldown_group(stack) else {
            return LocalItemUsePrediction::UnknownItemType(stack.item_id);
        };
        self.add_local_item_cooldown(
            cooldown_group,
            (instrument.use_duration * 20.0).floor() as i32,
        );
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::instrument_use(hand),
        ))
    }

    /// Resolves the effective `INSTRUMENT` component without interpreting a
    /// synchronized holder through the vanilla bootstrap ordinal.
    pub(crate) fn local_instrument_use_state(
        &self,
        stack: &ProtocolItemStackSummary,
    ) -> Option<LocalInstrumentUseState> {
        let effective = self.effective_item_stack(stack);
        match (
            effective.component_patch.instrument_id,
            effective.component_patch.instrument_direct.as_deref(),
        ) {
            (Some(holder_id), None) => {
                let instrument = self
                    .synchronized_instrument_presentation(holder_id)
                    .ok()??;
                let sound_event = match instrument.sound_event {
                    InstrumentSoundEvent::Reference { resource_id } => SoundEventSummary {
                        registry_id: Some(self.registry_holder_id_by_resource_id(
                            SOUND_EVENT_REGISTRY,
                            &resource_id,
                        )?),
                        sound_id: None,
                        fixed_range_bits: None,
                    },
                    InstrumentSoundEvent::Direct {
                        resource_id,
                        fixed_range,
                    } => SoundEventSummary {
                        registry_id: None,
                        sound_id: Some(resource_id),
                        fixed_range_bits: fixed_range.map(f32::to_bits),
                    },
                };
                Some(LocalInstrumentUseState {
                    sound_event,
                    use_duration: instrument.use_duration,
                    range: instrument.range,
                })
            }
            (None, Some(instrument)) => Some(local_direct_instrument_use_state(instrument)),
            _ => None,
        }
    }
}

fn local_direct_instrument_use_state(instrument: &InstrumentSummary) -> LocalInstrumentUseState {
    LocalInstrumentUseState {
        sound_event: instrument.sound_event.clone(),
        use_duration: f32::from_bits(instrument.use_duration_bits),
        range: f32::from_bits(instrument.range_bits),
    }
}
