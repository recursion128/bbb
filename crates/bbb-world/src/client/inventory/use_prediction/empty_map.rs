use super::*;

impl WorldStore {
    /// Predicts the logical-client half of `EmptyMapItem.use`.
    ///
    /// The client returns a client-swing `SUCCESS`; consumption, map creation,
    /// held-item transformation, inventory insertion/drop, sound, and stats are
    /// all inside vanilla's `ServerLevel` branch.
    pub(super) fn predict_local_empty_map_item_use(&self) -> LocalItemUsePrediction {
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::{
        item_types::VANILLA_ITEM_MAP_ID,
        packets::{ItemStackSummary, SetPlayerInventory},
    };

    #[test]
    fn empty_map_use_succeeds_without_local_stack_or_map_mutation() {
        let mut world = WorldStore::new();
        let map = ItemStackSummary {
            item_id: Some(VANILLA_ITEM_MAP_ID),
            count: 2,
            component_patch: Default::default(),
        };
        assert!(world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: map.clone(),
        }));

        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );
        assert_eq!(
            world.local_item_in_hand(InteractionHand::MainHand),
            Some(&map)
        );
        assert!(world.map_items().is_empty());
    }
}
