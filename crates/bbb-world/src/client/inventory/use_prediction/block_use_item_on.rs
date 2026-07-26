use super::{LocalBlockUseContext, LocalInteractionResult, LocalTargetUsePrediction};
use crate::WorldStore;
use bbb_protocol::{
    block_types::{BlockTypeCatalogEntry, VanillaBlockUseItemOnOverride},
    item_types::VANILLA_ITEM_GLOWSTONE_ID,
    packets::InteractionHand,
};

impl WorldStore {
    pub(super) fn predict_local_block_use_item_on(
        &mut self,
        owner: VanillaBlockUseItemOnOverride,
        block_type: &BlockTypeCatalogEntry,
        hand: InteractionHand,
        context: LocalBlockUseContext,
    ) -> LocalTargetUsePrediction {
        let unsupported = || LocalTargetUsePrediction::UnsupportedBlockUseItemOn(owner);
        let Some(block) = self.probe_block(context.pos) else {
            return unsupported();
        };
        if block.block_name.as_deref() != Some(block_type.resource_id) {
            return unsupported();
        }

        match owner {
            VanillaBlockUseItemOnOverride::ComposterBlock
            | VanillaBlockUseItemOnOverride::NoteBlock
            | VanillaBlockUseItemOnOverride::SweetBerryBushBlock
                if self.local_item_in_hand(hand).is_none() =>
            {
                try_empty_hand()
            }
            VanillaBlockUseItemOnOverride::LecternBlock
                if self.local_item_in_hand(hand).is_none() =>
            {
                match block.block_properties.get("has_book").map(String::as_str) {
                    Some("true") => try_empty_hand(),
                    Some("false") if hand == InteractionHand::MainHand => pass(),
                    Some("false") => try_empty_hand(),
                    _ => unsupported(),
                }
            }
            VanillaBlockUseItemOnOverride::RespawnAnchorBlock
                if self.local_item_in_hand(hand).is_none() =>
            {
                let charges = block
                    .block_properties
                    .get("charges")
                    .and_then(|charges| charges.parse::<u8>().ok())
                    .filter(|charges| *charges <= 4);
                match charges {
                    Some(charges)
                        if hand == InteractionHand::MainHand
                            && charges < 4
                            && self
                                .local_item_in_hand(InteractionHand::OffHand)
                                .and_then(|stack| stack.item_id)
                                == Some(VANILLA_ITEM_GLOWSTONE_ID) =>
                    {
                        pass()
                    }
                    Some(_) => try_empty_hand(),
                    None => unsupported(),
                }
            }
            _ => unsupported(),
        }
    }
}

fn pass() -> LocalTargetUsePrediction {
    LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
}

fn try_empty_hand() -> LocalTargetUsePrediction {
    LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
}
