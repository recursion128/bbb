use super::*;
use bbb_protocol::packets::ItemStackSummary;

impl WorldStore {
    /// Predicts the logical-client half of `KnowledgeBookItem.use`.
    ///
    /// Vanilla reads the effective recipes component and consumes the held
    /// stack before checking whether the list is empty. Recipe lookup,
    /// validation, awarding, and stats remain server-owned.
    pub(super) fn predict_local_knowledge_book_item_use(
        &mut self,
        hand: InteractionHand,
        stack: &ItemStackSummary,
    ) -> LocalItemUsePrediction {
        let has_recipes = if !self
            .item_stack_has_effective_component(stack, VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID)
        {
            false
        } else if stack
            .component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID)
        {
            let Some(recipe_keys) = stack.component_patch.recipe_keys.as_ref() else {
                return LocalItemUsePrediction::MissingComponentValue(
                    VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID,
                );
            };
            !recipe_keys.is_empty()
        } else {
            // The generated vanilla prototype owns an empty RECIPES value.
            false
        };

        if !self.consume_local_item_in_hand(hand, 1) {
            return LocalItemUsePrediction::UnsupportedOverride(
                VanillaItemUseOverride::KnowledgeBookItem,
            );
        }

        LocalItemUsePrediction::Result(if has_recipes {
            LocalInteractionResult::Success(LocalInteractionSuccess::client_item_interaction())
        } else {
            LocalInteractionResult::Fail
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::{
        item_types::VANILLA_ITEM_KNOWLEDGE_BOOK_ID,
        packets::{
            DataComponentPatchSummary, ItemStackSummary, PlayerAbilities, SetPlayerInventory,
        },
    };

    fn knowledge_book(recipe_keys: Option<Vec<&str>>) -> ItemStackSummary {
        let mut component_patch = DataComponentPatchSummary::default();
        if let Some(recipe_keys) = recipe_keys {
            component_patch.added = 1;
            component_patch
                .added_type_ids
                .push(VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID);
            component_patch.recipe_keys =
                Some(recipe_keys.into_iter().map(ToString::to_string).collect());
        }
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_KNOWLEDGE_BOOK_ID),
            count: 1,
            component_patch,
        }
    }

    fn set_main_hand(world: &mut WorldStore, stack: ItemStackSummary) {
        assert!(world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: stack,
        }));
    }

    #[test]
    fn knowledge_book_consumes_before_empty_recipe_failure() {
        for recipe_keys in [None, Some(Vec::new())] {
            let mut world = WorldStore::new();
            set_main_hand(&mut world, knowledge_book(recipe_keys));

            assert_eq!(
                world.predict_local_item_use(InteractionHand::MainHand),
                LocalItemUsePrediction::Result(LocalInteractionResult::Fail)
            );
            assert!(world
                .local_item_in_hand(InteractionHand::MainHand)
                .is_none());
        }
    }

    #[test]
    fn knowledge_book_with_recipes_consumes_and_client_swings() {
        let mut world = WorldStore::new();
        set_main_hand(&mut world, knowledge_book(Some(vec!["minecraft:bread"])));

        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );
        assert!(world
            .local_item_in_hand(InteractionHand::MainHand)
            .is_none());
    }

    #[test]
    fn knowledge_book_respects_component_removal_and_infinite_materials() {
        let mut world = WorldStore::new();
        let mut removed = knowledge_book(Some(vec!["minecraft:bread"]));
        removed
            .component_patch
            .removed_type_ids
            .push(VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID);
        set_main_hand(&mut world, removed.clone());
        world.apply_player_abilities(PlayerAbilities {
            invulnerable: false,
            flying: false,
            can_fly: false,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });

        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Fail)
        );
        assert_eq!(
            world.local_item_in_hand(InteractionHand::MainHand),
            Some(&removed)
        );
    }

    #[test]
    fn malformed_recipes_component_does_not_mutate_inventory() {
        let mut world = WorldStore::new();
        let mut malformed = knowledge_book(None);
        malformed.component_patch.added = 1;
        malformed
            .component_patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID);
        set_main_hand(&mut world, malformed.clone());

        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::MissingComponentValue(VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID)
        );
        assert_eq!(
            world.local_item_in_hand(InteractionHand::MainHand),
            Some(&malformed)
        );
    }
}
