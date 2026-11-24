use bbb_protocol::{
    data_component_types::VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
    packets::{InteractionHand, ItemStackSummary as ProtocolItemStackSummary, SoundEventSummary},
};

use crate::{registries::RegistrySoundEvent, WorldStore};

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
    ) -> LocalItemUsePrediction {
        let stack = self.local_item_in_hand(hand).unwrap();
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
        let patch = self.items.component_source(
            stack.item_id,
            &stack.component_patch,
            VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
        );
        let (sound_event, use_duration, range) = match (
            patch.instrument.registry_id,
            patch.instrument.direct.as_deref(),
        ) {
            (Some(holder_id), None) => {
                let instrument = self.synchronized_instrument_presentation(holder_id)?;
                let sound_event = match &instrument.sound_event {
                    RegistrySoundEvent::Reference { resource_id } => SoundEventSummary {
                        registry_id: Some(self.registry_holder_id_by_resource_id(
                            SOUND_EVENT_REGISTRY,
                            resource_id,
                        )?),
                        sound_id: None,
                        fixed_range_bits: None,
                    },
                    RegistrySoundEvent::Direct {
                        resource_id,
                        fixed_range,
                    } => SoundEventSummary {
                        registry_id: None,
                        sound_id: Some(resource_id.clone()),
                        fixed_range_bits: fixed_range.map(f32::to_bits),
                    },
                };
                (sound_event, instrument.use_duration, instrument.range)
            }
            (None, Some(instrument)) => (
                instrument.sound_event.clone(),
                f32::from_bits(instrument.use_duration_bits),
                f32::from_bits(instrument.range_bits),
            ),
            _ => return None,
        };
        Some(LocalInstrumentUseState {
            sound_event,
            use_duration,
            range,
        })
    }
}
