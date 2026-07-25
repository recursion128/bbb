use super::*;

impl WorldStore {
    /// Predicts the logical-client half of `FoodOnAStickItem.use`.
    ///
    /// Vanilla returns `PASS` before inspecting the controlled vehicle on the
    /// client. Boosting, durability/conversion, and stats are server-owned.
    pub(super) fn predict_local_food_on_a_stick_item_use(&self) -> LocalItemUsePrediction {
        LocalItemUsePrediction::Result(LocalInteractionResult::Pass)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::{
        item_types::{VANILLA_ITEM_CARROT_ON_A_STICK_ID, VANILLA_ITEM_WARPED_FUNGUS_ON_A_STICK_ID},
        packets::{ItemStackSummary, SetPlayerInventory},
    };

    #[test]
    fn generated_food_on_a_stick_owners_pass_without_local_mutation() {
        for item_id in [
            VANILLA_ITEM_CARROT_ON_A_STICK_ID,
            VANILLA_ITEM_WARPED_FUNGUS_ON_A_STICK_ID,
        ] {
            let mut world = WorldStore::new();
            let stack = ItemStackSummary {
                item_id: Some(item_id),
                count: 1,
                component_patch: Default::default(),
            };
            assert!(world.apply_set_player_inventory(SetPlayerInventory {
                slot: 0,
                item: stack.clone(),
            }));

            assert_eq!(
                world.predict_local_item_use(InteractionHand::MainHand),
                LocalItemUsePrediction::Result(LocalInteractionResult::Pass)
            );
            assert_eq!(
                world.local_item_in_hand(InteractionHand::MainHand),
                Some(&stack)
            );
        }
    }
}
