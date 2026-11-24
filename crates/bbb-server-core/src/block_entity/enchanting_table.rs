//! `EnchantingTableBlockEntity` (`EnchantingTableBlockEntity.java:19-48`).
//!
//! CustomName round-trips as opaque extra NBT. `bookAnimationTick` is client-only.

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::components::{ComponentMapBuilder, ComponentMapReader};
use super::{entry, unique_field};

const TAG_CUSTOM_NAME: &str = "CustomName";

/// `EnchantingTableBlockEntity` (`:19-35`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EnchantingTableBlockEntity {
    custom_name: Option<NbtSummaryValue>,
}

impl EnchantingTableBlockEntity {
    // EnchantingTableBlockEntity.java:123-126.
    pub(super) fn apply_implicit_components(
        &mut self,
        components: &mut ComponentMapReader<'_>,
    ) -> bbb_server_types::Result<()> {
        self.custom_name = components.read("minecraft:custom_name")?;
        Ok(())
    }

    // EnchantingTableBlockEntity.java:118-120.
    pub(super) fn custom_name(&self) -> Option<&NbtSummaryValue> {
        self.custom_name.as_ref()
    }

    /// `EnchantingTableBlockEntity.collectImplicitComponents` (`:129-132`).
    pub(super) fn collect_implicit_components(&self, components: &mut ComponentMapBuilder<'_>) {
        components.set("minecraft:custom_name", self.custom_name.clone());
    }

    /// `EnchantingTableBlockEntity.saveAdditional` (`:39-42`).
    pub(super) fn save_additional(&self) -> Vec<NbtSummaryEntry> {
        match &self.custom_name {
            Some(name) => vec![entry(TAG_CUSTOM_NAME, name.clone())],
            None => Vec::new(),
        }
    }

    /// `EnchantingTableBlockEntity.loadAdditional` (`:45-48`).
    pub(super) fn load_additional(&mut self, record: &[NbtSummaryEntry]) {
        self.custom_name = unique_field(record, TAG_CUSTOM_NAME).cloned();
    }
}

#[cfg(test)]
mod tests {

    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::VANILLA_BLOCK_ENCHANTING_TABLE_ID;
    use bbb_protocol::packets::BlockPos;
    use bbb_protocol::NbtSummaryValue;

    use super::{entry, TAG_CUSTOM_NAME};
    use crate::block_entity::{new_block_entity, unique_field};

    fn pos() -> BlockPos {
        BlockPos { x: 4, y: 5, z: 6 }
    }

    fn enchanting_table() -> i32 {
        vanilla_block_default_state_id(VANILLA_BLOCK_ENCHANTING_TABLE_ID)
            .expect("minecraft:enchanting_table has a default state")
    }

    fn custom_name() -> NbtSummaryValue {
        NbtSummaryValue::string("Enchant")
    }

    #[test]
    fn enchanting_table_save_load_round_trips_custom_name() {
        let state = enchanting_table();
        let mut record = new_block_entity(pos(), state)
            .expect("an enchanting table builds")
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        record.push(entry(TAG_CUSTOM_NAME, custom_name()));
        let loaded = crate::block_entity::load_test_fixture(pos(), state, &record)
            .expect("fixture codec succeeds")
            .expect("table loads");
        let saved = loaded
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        assert_eq!(unique_field(&saved, TAG_CUSTOM_NAME), Some(&custom_name()));
        assert!(loaded
            .update_tag_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds")
            .is_empty());
        assert!(
            crate::block_entity::block_entity_update_packet_with_resolver(
                &loaded,
                crate::block_entity::test_item_resolver()
            )
            .expect("fixture codec succeeds")
            .is_none()
        );
    }
}
