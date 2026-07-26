use std::collections::BTreeMap;

use bbb_protocol::{
    block_state_types::vanilla_item_default_block_state_properties,
    block_types::vanilla_block_type,
    data_component_types::VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID,
    item_types::{ItemTypeCatalogEntry, VanillaBlockItemKind, VanillaItemUseOnOverride},
    packets::{Direction as ProtocolDirection, ItemStackSummary as ProtocolItemStackSummary},
};

use super::*;

mod attachment;
mod composite;
mod connections;
mod context;
mod signal;
mod state;
pub(super) mod survival;

use state::BlockPlacementState;

impl WorldStore {
    pub(super) fn predict_local_block_item_use_on(
        &mut self,
        owner: VanillaItemUseOnOverride,
        item_type: &ItemTypeCatalogEntry,
        hand: InteractionHand,
        context: LocalBlockUseContext,
    ) -> LocalTargetUsePrediction {
        let unsupported = || LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        let Some(stack) = self.local_item_in_hand(hand).cloned() else {
            return unsupported();
        };
        if stack.item_id != Some(item_type.id) || stack.count < 1 {
            return unsupported();
        }
        if !context
            .cursor
            .iter()
            .all(|coordinate| coordinate.is_finite())
        {
            return unsupported();
        }
        // Base `Block.getStateForPlacement` does not inspect `isInside`; the
        // exact value remains in the context for specialized state owners.
        let _inside = context.inside;
        let Some(behavior) = item_type.block_item_behavior else {
            return unsupported();
        };

        match behavior.kind {
            VanillaBlockItemKind::BlockItem
            | VanillaBlockItemKind::BedItem
            | VanillaBlockItemKind::BannerItem
            | VanillaBlockItemKind::DoubleHighBlockItem
            | VanillaBlockItemKind::PlayerHeadItem
            | VanillaBlockItemKind::SignItem
            | VanillaBlockItemKind::StandingAndWallBlockItem => {}
            VanillaBlockItemKind::GameMasterBlockItem => {
                if !self.local_player_has_gamemaster_permission() {
                    return block_item_fallback(self, hand, &stack, LocalInteractionResult::Fail);
                }
            }
            _ => return unsupported(),
        }
        let Some(block_type) = vanilla_block_type(behavior.block_type_id) else {
            return unsupported();
        };
        let Some(replace_clicked) =
            state::block_item_can_replace(self, context.pos, block_type, context, true)
        else {
            return unsupported();
        };
        let target_pos = if replace_clicked {
            context.pos
        } else {
            relative(context.pos, context.face)
        };
        if !replace_clicked {
            let Some(replace_target) =
                state::block_item_can_replace(self, target_pos, block_type, context, false)
            else {
                return unsupported();
            };
            if !replace_target {
                return block_item_fallback(self, hand, &stack, LocalInteractionResult::Fail);
            }
        }

        let standing_and_wall = matches!(
            behavior.kind,
            VanillaBlockItemKind::BannerItem
                | VanillaBlockItemKind::PlayerHeadItem
                | VanillaBlockItemKind::SignItem
                | VanillaBlockItemKind::StandingAndWallBlockItem
        );
        let placement = if standing_and_wall {
            attachment::standing_and_wall_state_for_placement(
                self,
                behavior,
                target_pos,
                replace_clicked,
                context,
                ProtocolDirection::Down,
            )
        } else {
            state::block_state_for_placement(self, block_type, target_pos, replace_clicked, context)
        };
        let placement_state_id = match placement {
            BlockPlacementState::State(state_id) => state_id,
            BlockPlacementState::Fail => {
                return block_item_fallback(self, hand, &stack, LocalInteractionResult::Fail);
            }
            BlockPlacementState::Unsupported => return unsupported(),
        };
        let Some(placement_state) = self.registries().block_state(placement_state_id) else {
            return unsupported();
        };
        let Some(placed_block_type) =
            bbb_protocol::block_types::vanilla_block_type_for_resource_id(&placement_state.name)
        else {
            return unsupported();
        };
        let expected_placement_block = placed_block_type.id == block_type.id
            || behavior
                .wall_block_type_id
                .is_some_and(|wall_block_type_id| wall_block_type_id == placed_block_type.id);
        if !expected_placement_block {
            return unsupported();
        }
        if let Some(owner) = placed_block_type.can_survive_override {
            match self.local_block_state_can_survive(target_pos, placement_state_id, owner) {
                Some(true) => {}
                Some(false) => {
                    return block_item_fallback(self, hand, &stack, LocalInteractionResult::Fail);
                }
                None => return unsupported(),
            }
        }
        let unobstructed = if standing_and_wall {
            self.local_block_state_empty_context_is_unobstructed(target_pos, placement_state_id)
        } else {
            self.local_block_state_placement_is_unobstructed(target_pos, placement_state_id)
        };
        let Some(unobstructed) = unobstructed else {
            return unsupported();
        };
        if !unobstructed {
            return block_item_fallback(self, hand, &stack, LocalInteractionResult::Fail);
        }

        let final_state_id =
            match self.block_item_state_after_component(item_type.id, &stack, placement_state_id) {
                Some(state_id) => state_id,
                None => return unsupported(),
            };
        let mut predicted_states = vec![(target_pos, final_state_id)];
        if behavior.kind == VanillaBlockItemKind::DoubleHighBlockItem {
            let Some(upper_state_id) = composite::double_high_client_upper_state(
                self,
                block_type,
                target_pos,
                final_state_id,
            ) else {
                return unsupported();
            };
            predicted_states.push((relative(target_pos, ProtocolDirection::Up), upper_state_id));
        }
        if !self.predict_local_block_states(&predicted_states, context.sequence) {
            return unsupported();
        }
        // The cloned non-empty hand stack above freezes the same precondition
        // consumed by ItemStack.consume after the successful setBlock.
        if !self.consume_local_item_in_hand(hand, 1) {
            return unsupported();
        }

        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::block_place_from_profile(target_pos, placed_block_type.id),
        ))
    }

    fn block_item_state_after_component(
        &self,
        item_type_id: i32,
        stack: &ProtocolItemStackSummary,
        placement_state_id: i32,
    ) -> Option<i32> {
        let properties = block_state_component_properties(item_type_id, stack);
        if properties.is_empty() {
            return Some(placement_state_id);
        }
        let placement_state = self.registries().block_state(placement_state_id)?;
        let block_name = placement_state.name.clone();
        let mut effective_properties = placement_state.properties.clone();
        let mut state_id = placement_state_id;
        for (name, value) in properties {
            let Some(previous) = effective_properties.get(&name).cloned() else {
                continue;
            };
            effective_properties.insert(name.clone(), value);
            if let Some(candidate) = self
                .registries()
                .block_state_id_by_name_and_properties(&block_name, &effective_properties)
            {
                state_id = candidate;
            } else {
                effective_properties.insert(name, previous);
            }
        }
        Some(state_id)
    }
}

fn block_item_fallback(
    world: &WorldStore,
    hand: InteractionHand,
    stack: &ProtocolItemStackSummary,
    placement_result: LocalInteractionResult,
) -> LocalTargetUsePrediction {
    if !world.item_stack_has_effective_component(
        stack,
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
    ) {
        return LocalTargetUsePrediction::Result(placement_result);
    }
    if !world.local_item_in_hand_can_start_consuming(hand) {
        return LocalTargetUsePrediction::Result(LocalInteractionResult::Fail);
    }
    let Some(consumable) = world.effective_item_consumable(stack) else {
        return LocalTargetUsePrediction::UnsupportedItemUseOn(VanillaItemUseOnOverride::BlockItem);
    };
    LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
        LocalInteractionSuccess::consume(consumable.consume_seconds > 0.0),
    ))
}

fn block_state_component_properties(
    item_type_id: i32,
    stack: &ProtocolItemStackSummary,
) -> BTreeMap<String, String> {
    if stack
        .component_patch
        .removed_type_ids
        .contains(&VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID)
    {
        return BTreeMap::new();
    }
    if stack
        .component_patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID)
    {
        return stack.component_patch.block_state_properties.clone();
    }
    vanilla_item_default_block_state_properties(item_type_id)
        .unwrap_or_default()
        .iter()
        .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
        .collect()
}

fn relative(pos: BlockPos, direction: ProtocolDirection) -> BlockPos {
    match direction {
        ProtocolDirection::Down => BlockPos {
            y: pos.y - 1,
            ..pos
        },
        ProtocolDirection::Up => BlockPos {
            y: pos.y + 1,
            ..pos
        },
        ProtocolDirection::North => BlockPos {
            z: pos.z - 1,
            ..pos
        },
        ProtocolDirection::South => BlockPos {
            z: pos.z + 1,
            ..pos
        },
        ProtocolDirection::West => BlockPos {
            x: pos.x - 1,
            ..pos
        },
        ProtocolDirection::East => BlockPos {
            x: pos.x + 1,
            ..pos
        },
    }
}

#[cfg(test)]
mod tests;
