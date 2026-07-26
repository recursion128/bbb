use super::*;
use bbb_protocol::{
    entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, item_types::VANILLA_ITEM_ARMOR_STAND_ID,
};

impl WorldStore {
    pub(super) fn predict_local_armor_stand_item_use_on(
        &mut self,
        owner: VanillaItemUseOnOverride,
        hand: InteractionHand,
        context: LocalBlockUseContext,
    ) -> LocalTargetUsePrediction {
        let unresolved_owner = || LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        let Some(stack) = self.local_item_in_hand(hand) else {
            return unresolved_owner();
        };
        if stack.item_id != Some(VANILLA_ITEM_ARMOR_STAND_ID) || stack.count < 1 {
            return unresolved_owner();
        }
        if context.face == ProtocolDirection::Down {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Fail);
        }

        let Some(replace_clicked) = self.local_block_is_replaceable_by_different_item(context.pos)
        else {
            return unresolved_owner();
        };
        let target_pos = if replace_clicked {
            context.pos
        } else {
            relative(context.pos, context.face)
        };
        let Some(bounds) = vanilla_bounds_for_type(VANILLA_ENTITY_TYPE_ARMOR_STAND_ID) else {
            return unresolved_owner();
        };
        let origin = [
            f64::from(target_pos.x) + 0.5,
            f64::from(target_pos.y),
            f64::from(target_pos.z) + 0.5,
        ];
        let query_min = std::array::from_fn(|axis| origin[axis] + f64::from(bounds.min[axis]));
        let query_max = std::array::from_fn(|axis| origin[axis] + f64::from(bounds.max[axis]));

        match self.local_empty_context_block_bounds_are_free(query_min, query_max) {
            Some(true) => {}
            Some(false) => {
                return LocalTargetUsePrediction::Result(LocalInteractionResult::Fail);
            }
            None => return unresolved_owner(),
        }
        match self.local_non_spectator_entity_bounds_are_empty(query_min, query_max) {
            Some(true) => {}
            Some(false) => {
                return LocalTargetUsePrediction::Result(LocalInteractionResult::Fail);
            }
            None => return unresolved_owner(),
        }
        if !self.shrink_local_item_in_hand(hand, 1) {
            return unresolved_owner();
        }
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction(),
        ))
    }
}

fn relative(pos: BlockPos, direction: ProtocolDirection) -> BlockPos {
    match direction {
        ProtocolDirection::Down => BlockPos {
            y: pos.y.saturating_sub(1),
            ..pos
        },
        ProtocolDirection::Up => BlockPos {
            y: pos.y.saturating_add(1),
            ..pos
        },
        ProtocolDirection::North => BlockPos {
            z: pos.z.saturating_sub(1),
            ..pos
        },
        ProtocolDirection::South => BlockPos {
            z: pos.z.saturating_add(1),
            ..pos
        },
        ProtocolDirection::West => BlockPos {
            x: pos.x.saturating_sub(1),
            ..pos
        },
        ProtocolDirection::East => BlockPos {
            x: pos.x.saturating_add(1),
            ..pos
        },
    }
}
