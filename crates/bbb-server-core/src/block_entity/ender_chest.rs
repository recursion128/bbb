//! `EnderChestBlockEntity` (`EnderChestBlockEntity.java:13-102`).
//!
//! Items live on the player. The server's extra state is the lid flag
//! `triggerEvent` writes and the opener count its own
//! `ContainerOpenersCounter` (`:15-53`) keeps, which no save carries because
//! `EnderChestBlockEntity` declares no `saveAdditional`.

/// `EnderChestBlockEntity` (`:13-57`) without the client lid animation.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EnderChestBlockEntity {
    /// `ChestLidController.shouldBeOpen` (`ChestLidController.java:6`).
    should_be_open: bool,
    /// `ContainerOpenersCounter.openCount` (`ContainerOpenersCounter.java:17`).
    open_count: i32,
    max_interaction_range: f64,
}

impl EnderChestBlockEntity {
    #[must_use]
    pub const fn should_be_open(&self) -> bool {
        self.should_be_open
    }

    /// `ContainerOpenersCounter.getOpenerCount` (`:96-98`).
    #[must_use]
    pub const fn open_count(&self) -> i32 {
        self.open_count
    }

    /// `ContainerOpenersCounter.recheckOpeners` (`:66-94`) count write.
    pub fn set_open_count(&mut self, count: i32) {
        self.open_count = count;
    }

    pub fn max_interaction_range(&self) -> f64 {
        self.max_interaction_range
    }

    pub fn set_max_interaction_range(&mut self, range: f64) {
        self.max_interaction_range = range;
    }

    /// `EnderChestBlockEntity.triggerEvent` (`EnderChestBlockEntity.java:64-70`).
    #[must_use]
    pub fn trigger_event(&mut self, param_a: i32, param_b: i32) -> bool {
        if param_a != 1 {
            return false;
        }
        self.should_be_open = param_b > 0;
        true
    }
}

#[cfg(test)]
mod tests {

    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::VANILLA_BLOCK_ENDER_CHEST_ID;
    use bbb_protocol::packets::BlockPos;

    use crate::block_entity::{new_block_entity, BlockEntityPayload};

    use super::EnderChestBlockEntity;

    fn pos() -> BlockPos {
        BlockPos { x: 1, y: 2, z: 3 }
    }

    fn default_state() -> i32 {
        vanilla_block_default_state_id(VANILLA_BLOCK_ENDER_CHEST_ID)
            .expect("minecraft:ender_chest has a default state")
    }

    #[test]
    fn trigger_event_one_sets_the_lid_from_param_b() {
        let mut lid = EnderChestBlockEntity::default();
        assert!(lid.trigger_event(1, 3));
        assert!(lid.should_be_open());
        assert!(lid.trigger_event(1, 0));
        assert!(!lid.should_be_open());
        assert!(!lid.trigger_event(0, 1));
        assert!(!lid.should_be_open());

        let mut built =
            new_block_entity(pos(), default_state()).expect("an ender chest builds an object");
        assert!(!built.trigger_event(2, 1));
        assert!(built.trigger_event(1, 1));
        let BlockEntityPayload::EnderChest(chest) = built.payload() else {
            panic!("payload is an ender chest");
        };
        assert!(chest.should_be_open());
        assert!(built.trigger_event(1, 0));
        let BlockEntityPayload::EnderChest(chest) = built.payload() else {
            panic!("payload is an ender chest");
        };
        assert!(!chest.should_be_open());
    }
}
