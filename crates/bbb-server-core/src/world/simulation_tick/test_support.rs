//! Test-only hooks over the world state the simulation tick owns: the store
//! setters and the TNT fixtures the simulation tests drive the world through.

use super::*;

#[cfg(test)]
impl ServerWorld {
    pub(crate) fn tick_trial_spawner_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        pos: BlockPos,
    ) {
        self.tick_trial_spawner(&dimension.into(), pos, &mut Vec::new());
    }

    pub(crate) fn set_inventory_slot_for_test(
        &mut self,
        connection_id: ConnectionId,
        slot: usize,
        item: bbb_protocol::packets::ItemStackSummary,
    ) {
        *self.levels[connection_id]
            .inventory
            .slot_mut(slot)
            .expect("the test inventory slot is addressable") =
            bbb_server_types::item_stack::RuntimeItemStack::from_summary(item);
    }

    pub(crate) fn set_ender_chest_slot_for_test(
        &mut self,
        connection_id: ConnectionId,
        slot: usize,
        item: bbb_protocol::packets::ItemStackSummary,
    ) {
        assert!(
            self.levels[connection_id]
                .inventory
                .set_ender_chest_slot(slot, item),
            "the test ender chest slot is addressable"
        );
    }

    pub(crate) fn player_inventory_for_test(
        &mut self,
        connection_id: ConnectionId,
    ) -> &mut crate::inventory::PlayerInventoryMenu {
        &mut self.levels[connection_id].inventory
    }

    pub(crate) fn ender_chest_slot_for_test(
        &self,
        connection_id: ConnectionId,
        slot: usize,
    ) -> bbb_protocol::packets::ItemStackSummary {
        self.levels[connection_id].inventory.ender_chest_slots()[slot]
            .summary()
            .clone()
    }

    pub(crate) fn remove_stored_entity_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        entity_id: i32,
        reason: crate::entity::RemovalReason,
    ) -> bool {
        self.level_mut(dimension)
            .entities
            .remove(entity_id, reason)
            .is_some()
    }

    pub(crate) fn mark_avatar_removed_for_test(&mut self, connection_id: ConnectionId) {
        self.levels[connection_id].avatar_removed = true;
    }

    pub(crate) fn set_stored_entity_position_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        entity_id: i32,
        position: Vec3d,
    ) {
        self.level_mut(dimension)
            .entities
            .get_mut(entity_id)
            .expect("the test entity is live")
            .set_pos(position);
    }

    /// `Entity.setPos` with the section callback it ends in
    /// (`PersistentEntitySectionManager.java:377-392`).
    pub(crate) fn move_stored_entity_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        entity_id: i32,
        position: Vec3d,
    ) {
        self.set_stored_entity_position_for_test(dimension, entity_id, position);
        self.level_mut(dimension).entities.refile(entity_id);
    }
}

impl ServerWorld {
    #[cfg(test)]
    pub(crate) fn set_tnt_explodes_for_test(&mut self, value: bool) {
        self.game_rules.tnt_explodes = value;
    }
}
