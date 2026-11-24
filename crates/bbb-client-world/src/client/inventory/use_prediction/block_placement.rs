use bbb_protocol::{
    block_state_types::{
        vanilla_block_state_with_property, vanilla_item_default_block_state_properties,
    },
    block_types::{
        vanilla_block_type, vanilla_concrete_block_type_id_for_powder,
        VanillaGetStateForPlacementOverride, VANILLA_BLOCK_BAMBOO_SAPLING_ID,
        VANILLA_BLOCK_DIRT_ID,
    },
    data_component_types::VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID,
    item_types::{ItemTypeCatalogEntry, VanillaBlockItemKind, VanillaItemUseOnOverride},
    packets::{Direction as ProtocolDirection, ItemStackSummary as ProtocolItemStackSummary},
};

use super::*;
use crate::{registries::BlockStateInfo, LevelEventSoundRandomState};

mod attachment;
mod composite;
mod connections;
mod context;
mod fluid_contact;
mod scaffolding;
mod signal;
mod state;
mod surface;
pub(super) mod survival;

use state::BlockPlacementState;

impl WorldStore {
    /// Mirrors the `BlockPlaceContext.canPlace` query used by block callbacks
    /// that decide whether to pass to a held `BlockItem`.
    pub(super) fn local_block_item_context_can_place(
        &self,
        item_type: &ItemTypeCatalogEntry,
        context: LocalBlockUseContext,
    ) -> Option<bool> {
        let behavior = item_type.block_item_behavior?;
        let block_type = vanilla_block_type(behavior.block_type_id)?;
        let replace_clicked =
            state::block_item_can_replace(self, context.pos, block_type, context, true)?;
        if replace_clicked {
            return Some(true);
        }
        state::block_item_can_replace(
            self,
            relative(context.pos, context.face),
            block_type,
            context,
            false,
        )
    }

    pub(super) fn predict_local_block_item_use_on(
        &mut self,
        owner: VanillaItemUseOnOverride,
        item_type: &ItemTypeCatalogEntry,
        hand: InteractionHand,
        context: LocalBlockUseContext,
        random: Option<&mut LevelEventSoundRandomState>,
    ) -> LocalTargetUsePrediction {
        let unsupported = || LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        let Some(stack) = self.local_item_in_hand(hand) else {
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
            | VanillaBlockItemKind::HangingSignItem
            | VanillaBlockItemKind::PlayerHeadItem
            | VanillaBlockItemKind::ScaffoldingBlockItem
            | VanillaBlockItemKind::SignItem
            | VanillaBlockItemKind::StandingAndWallBlockItem => {}
            VanillaBlockItemKind::GameMasterBlockItem => {
                if !self.local_player_can_use_gamemaster_blocks() {
                    return block_item_fallback(self, hand, stack, LocalInteractionResult::Fail);
                }
            }
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
                return block_item_fallback(self, hand, stack, LocalInteractionResult::Fail);
            }
        }
        let (target_pos, replace_clicked, context) = if behavior.kind
            == VanillaBlockItemKind::ScaffoldingBlockItem
        {
            match scaffolding::update_placement_context(self, context, target_pos, replace_clicked)
            {
                scaffolding::ScaffoldingContextUpdate::Context(updated) => {
                    (updated.target_pos, updated.replace_clicked, updated.context)
                }
                scaffolding::ScaffoldingContextUpdate::Fail => {
                    return block_item_fallback(self, hand, stack, LocalInteractionResult::Fail);
                }
                scaffolding::ScaffoldingContextUpdate::Unsupported => return unsupported(),
            }
        } else {
            (target_pos, replace_clicked, context)
        };

        let standing_and_wall = matches!(
            behavior.kind,
            VanillaBlockItemKind::BannerItem
                | VanillaBlockItemKind::HangingSignItem
                | VanillaBlockItemKind::PlayerHeadItem
                | VanillaBlockItemKind::SignItem
                | VanillaBlockItemKind::StandingAndWallBlockItem
        );
        let placement = if standing_and_wall {
            let attachment_direction = if behavior.kind == VanillaBlockItemKind::HangingSignItem {
                ProtocolDirection::Up
            } else {
                ProtocolDirection::Down
            };
            attachment::standing_and_wall_state_for_placement(
                self,
                behavior,
                target_pos,
                replace_clicked,
                context,
                attachment_direction,
            )
        } else {
            state::block_state_for_placement(
                self,
                block_type,
                target_pos,
                replace_clicked,
                context,
                random,
            )
        };
        let placement_state_id = match placement {
            BlockPlacementState::State(state_id) => state_id,
            BlockPlacementState::Fail => {
                return block_item_fallback(self, hand, stack, LocalInteractionResult::Fail);
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
                .is_some_and(|wall_block_type_id| wall_block_type_id == placed_block_type.id)
            || block_type.get_state_for_placement_override
                == Some(VanillaGetStateForPlacementOverride::BambooStalkBlock)
                && placed_block_type.id == VANILLA_BLOCK_BAMBOO_SAPLING_ID
            || vanilla_concrete_block_type_id_for_powder(block_type.id)
                == Some(placed_block_type.id)
            || matches!(
                block_type.get_state_for_placement_override,
                Some(
                    VanillaGetStateForPlacementOverride::DirtPathBlock
                        | VanillaGetStateForPlacementOverride::FarmlandBlock
                )
            ) && placed_block_type.id == VANILLA_BLOCK_DIRT_ID
            || block_type.get_state_for_placement_override
                == Some(VanillaGetStateForPlacementOverride::CopperChestBlock)
                && placed_block_type.get_state_for_placement_override
                    == Some(VanillaGetStateForPlacementOverride::CopperChestBlock)
            || state::growing_plant_body_block_type_id_for_head(block_type.id)
                == Some(placed_block_type.id);
        if !expected_placement_block {
            return unsupported();
        }
        if behavior.kind != VanillaBlockItemKind::ScaffoldingBlockItem {
            if let Some(owner) = placed_block_type.can_survive_override {
                match self.local_block_state_can_survive(target_pos, placement_state_id, owner) {
                    Some(true) => {}
                    Some(false) => {
                        return block_item_fallback(
                            self,
                            hand,
                            stack,
                            LocalInteractionResult::Fail,
                        );
                    }
                    None => return unsupported(),
                }
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
            return block_item_fallback(self, hand, stack, LocalInteractionResult::Fail);
        }

        let final_state_id =
            Self::block_item_state_after_component(item_type.id, stack, placement_state);
        let mut predicted_states = vec![(target_pos, final_state_id)];
        if block_type.get_state_for_placement_override
            == Some(VanillaGetStateForPlacementOverride::TripWireHookBlock)
        {
            let Some(placement) =
                composite::trip_wire_hook_client_placement(self, target_pos, final_state_id)
            else {
                return unsupported();
            };
            predicted_states = placement;
        }
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
        if !self.predict_local_player_block_states(&predicted_states, context.sequence) {
            return unsupported();
        }
        if !self.consume_local_item_in_hand(hand, 1) {
            return unsupported();
        }

        let success =
            LocalInteractionSuccess::block_place_from_profile(target_pos, placed_block_type.id);
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(success))
    }

    fn block_item_state_after_component(
        item_type_id: i32,
        stack: &ProtocolItemStackSummary,
        placement_state: &BlockStateInfo,
    ) -> i32 {
        let patch = &stack.component_patch;
        if patch
            .removed_type_ids
            .contains(&VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID)
        {
            return placement_state.id;
        }
        let explicit = patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID);
        let defaults = if explicit {
            &[][..]
        } else {
            vanilla_item_default_block_state_properties(item_type_id).unwrap_or_default()
        };
        if (explicit && patch.block_state_properties.is_empty())
            || (!explicit && defaults.is_empty())
        {
            return placement_state.id;
        }

        let mut state_id = placement_state.id;
        let mut apply_property = |name: &str, value: &str| {
            if let Some(candidate) = vanilla_block_state_with_property(state_id, name, value) {
                state_id = candidate;
            }
        };
        for name in placement_state.properties.keys() {
            if explicit {
                if let Some(value) = patch.block_state_properties.get_str(name) {
                    let _ = value.with_utf8(|value| apply_property(name, value));
                }
            } else if let Some((_, value)) = defaults.iter().find(|(key, _)| *key == name.as_str())
            {
                apply_property(name, value);
            }
        }
        state_id
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
        LocalInteractionSuccess::start_consuming(hand, consumable),
    ))
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
