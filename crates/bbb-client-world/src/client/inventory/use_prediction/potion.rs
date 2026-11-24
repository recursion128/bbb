use bbb_protocol::{
    block_state_types::vanilla_block_default_state_id,
    block_types::{vanilla_block_type_for_resource_id, VANILLA_BLOCK_MUD_ID},
    data_component_types::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
    item_types::{VanillaItemUseOnOverride, VANILLA_ITEM_GLASS_BOTTLE_ID, VANILLA_ITEM_POTION_ID},
    packets::{Direction as ProtocolDirection, InteractionHand, ItemStackSummary},
    potion_types::vanilla_potion,
};

use super::{
    CreativeFilledResult, LocalBlockUseContext, LocalInteractionResult, LocalInteractionSuccess,
    LocalTargetUsePrediction,
};
use crate::WorldStore;

const CONVERTABLE_TO_MUD_TAG: &str = "minecraft:convertable_to_mud";
const WATER_POTION: &str = "minecraft:water";

impl WorldStore {
    /// Mirrors the client-owned part of `PotionItem.useOn`.
    ///
    /// A water potion used on a live `convertable_to_mud` block replaces the
    /// held result with a glass bottle and predicts the mud default state.
    /// Splash particles remain server-only.
    pub(super) fn predict_local_potion_item_use_on(
        &mut self,
        owner: VanillaItemUseOnOverride,
        hand: InteractionHand,
        context: LocalBlockUseContext,
    ) -> LocalTargetUsePrediction {
        if context.face == ProtocolDirection::Down {
            return pass();
        }

        let Some(block) = self.block_view(context.pos) else {
            return LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        };
        let Some(block_name) = block.block_name else {
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

        let Some(stack) = self.local_item_in_hand(hand) else {
            return LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        };
        if stack.item_id != Some(VANILLA_ITEM_POTION_ID) || stack.count < 1 {
            return LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        }
        if !self.item_stack_has_effective_component(
            stack,
            VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
        ) {
            return pass();
        }
        let patch = self.items.component_source(
            stack.item_id,
            &stack.component_patch,
            VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
        );
        let Some(potion_id) = patch.potion_contents.potion_id else {
            return pass();
        };
        let Some(potion) = vanilla_potion(potion_id) else {
            return LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        };
        if potion.resource_id != WATER_POTION {
            return pass();
        }

        let Some(mud_state_id) = vanilla_block_default_state_id(VANILLA_BLOCK_MUD_ID) else {
            return LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        };
        if !self.predict_local_block_state(context.pos, mud_state_id, context.sequence) {
            return LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        }
        let Some(dropped) = self.replace_local_item_with_filled_result(
            CreativeFilledResult::AddIfMissing,
            hand,
            VANILLA_ITEM_POTION_ID,
            ItemStackSummary {
                item_id: Some(VANILLA_ITEM_GLASS_BOTTLE_ID),
                count: 1,
                component_patch: Default::default(),
            },
        ) else {
            return LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        };

        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction_with_drop(dropped),
        ))
    }
}

fn pass() -> LocalTargetUsePrediction {
    LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
}
