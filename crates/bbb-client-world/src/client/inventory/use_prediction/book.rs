use super::*;

impl WorldStore {
    /// Predicts the shared logical-client result of writable and written book
    /// use. Writable content opens the local editor in the runtime; written
    /// content waits for the server's `OpenBook` packet.
    pub(super) fn predict_local_book_item_use(&self) -> LocalItemUsePrediction {
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::{
        item_types::{VANILLA_ITEM_WRITABLE_BOOK_ID, VANILLA_ITEM_WRITTEN_BOOK_ID},
        packets::{ItemStackSummary, SetPlayerInventory},
    };

    #[test]
    fn writable_and_written_book_use_are_client_swing_successes() {
        for item_id in [VANILLA_ITEM_WRITABLE_BOOK_ID, VANILLA_ITEM_WRITTEN_BOOK_ID] {
            let mut world = WorldStore::new();
            assert!(world.apply_set_player_inventory(SetPlayerInventory {
                slot: 0,
                item: ItemStackSummary {
                    item_id: Some(item_id),
                    count: 1,
                    component_patch: Default::default(),
                },
            }));

            assert_eq!(
                world.predict_local_item_use(InteractionHand::MainHand),
                LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                    LocalInteractionSuccess::client_item_interaction()
                ))
            );
            assert!(!world.local_player().interaction.using_item);
            assert_eq!(
                world
                    .local_item_in_hand(InteractionHand::MainHand)
                    .and_then(|stack| stack.item_id),
                Some(item_id)
            );
        }
    }
}
