use std::collections::BTreeMap;

use bbb_protocol::{
    block_types::{vanilla_block_type_for_resource_id, VANILLA_BLOCK_MUD_ID},
    data_component_types::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
    item_types::{VanillaItemUseOnOverride, VANILLA_ITEM_POTION_ID},
    packets::{Direction as ProtocolDirection, InteractionHand},
    potion_types::vanilla_potion,
};

use super::{
    LocalBlockUseContext, LocalInteractionResult, LocalInteractionSuccess, LocalTargetUsePrediction,
};
use crate::WorldStore;

const CONVERTABLE_TO_MUD_TAG: &str = "minecraft:convertable_to_mud";
const WATER_POTION: &str = "minecraft:water";

impl WorldStore {
    /// Mirrors the client-owned part of `PotionItem.useOn`.
    ///
    /// A water potion used on a live `convertable_to_mud` block replaces the
    /// held result with a glass bottle, predicts the mud default state, and
    /// consumes two suppressed local sound seeds. Splash particles remain
    /// server-only.
    pub(super) fn predict_local_potion_item_use_on(
        &mut self,
        owner: VanillaItemUseOnOverride,
        hand: InteractionHand,
        context: LocalBlockUseContext,
    ) -> LocalTargetUsePrediction {
        if context.face == ProtocolDirection::Down {
            return pass();
        }

        let Some(block) = self.probe_block(context.pos) else {
            return LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        };
        let Some(block_name) = block.block_name.as_deref() else {
            return LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        };
        let Some(block_type) = vanilla_block_type_for_resource_id(block_name) else {
            return LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        };
        let Some(convertable_blocks) =
            self.registry_holder_ids_for_tag("minecraft:block", CONVERTABLE_TO_MUD_TAG)
        else {
            return LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        };
        if !convertable_blocks.contains(&block_type.id) {
            return pass();
        }

        let Some(stack) = self.local_item_in_hand(hand).cloned() else {
            return LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        };
        if stack.item_id != Some(VANILLA_ITEM_POTION_ID) || stack.count < 1 {
            return LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        }
        if !self.item_stack_has_effective_component(
            &stack,
            VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
        ) {
            return pass();
        }
        let effective = self.effective_item_stack(&stack);
        let Some(potion_id) = effective.component_patch.potion_id else {
            return pass();
        };
        let Some(potion) = vanilla_potion(potion_id) else {
            return LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        };
        if potion.resource_id != WATER_POTION {
            return pass();
        }

        let Some(mud_state_id) = self
            .registries()
            .block_state_id_by_name_and_properties("minecraft:mud", &BTreeMap::new())
        else {
            return LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        };
        let Some(mud_type) = self
            .registries()
            .block_state(mud_state_id)
            .and_then(|state| vanilla_block_type_for_resource_id(&state.name))
        else {
            return LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        };
        if mud_type.id != VANILLA_BLOCK_MUD_ID
            || !self.predict_local_block_state(context.pos, mud_state_id, context.sequence)
        {
            return LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        }
        let Some(dropped) = self.replace_local_potion_with_glass_bottle(hand) else {
            return LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        };

        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::potion_mud_conversion(dropped),
        ))
    }
}

fn pass() -> LocalTargetUsePrediction {
    LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
}
