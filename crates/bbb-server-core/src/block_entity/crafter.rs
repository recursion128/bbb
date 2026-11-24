//! `CrafterBlockEntity` (`CrafterBlockEntity.java:25-267`).

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::Result;

use super::chest::{load_items, saved_items, TAG_ITEMS};
use super::components::{ComponentMapBuilder, ComponentMapReader};
use super::container_fields::{ContainerLootTable, ContainerNameAndLock};
use super::value_input::int_or;
use super::{entry, unique_field, BlockEntityItemLoad, BlockEntityItemSave};
use bbb_server_types::item_stack::RuntimeItemStack;

/// `CrafterBlockEntity.CONTAINER_SIZE` (`CrafterBlockEntity.java:28`).
pub const CRAFTER_CONTAINER_SIZE: usize = 9;
const TAG_CRAFTING_TICKS_REMAINING: &str = "crafting_ticks_remaining";
const TAG_DISABLED_SLOTS: &str = "disabled_slots";
const TAG_TRIGGERED: &str = "triggered";

/// `CrafterBlockEntity` (`:25-64`).
#[derive(Debug, Clone, PartialEq)]
pub struct CrafterBlockEntity {
    items: Vec<RuntimeItemStack>,
    crafting_ticks_remaining: i32,
    disabled: [bool; CRAFTER_CONTAINER_SIZE],
    triggered: i32,
    base: ContainerNameAndLock,
    loot_table: ContainerLootTable,
}

impl Default for CrafterBlockEntity {
    fn default() -> Self {
        Self {
            items: vec![RuntimeItemStack::empty(); CRAFTER_CONTAINER_SIZE],
            crafting_ticks_remaining: 0,
            disabled: [false; CRAFTER_CONTAINER_SIZE],
            triggered: 0,
            base: ContainerNameAndLock::default(),
            loot_table: ContainerLootTable::default(),
        }
    }
}

impl CrafterBlockEntity {
    /// `RandomizableContainerBlockEntity.collectImplicitComponents`
    /// (`RandomizableContainerBlockEntity.java:107-112`).
    pub(super) fn collect_implicit_components(&self, components: &mut ComponentMapBuilder<'_>) {
        self.base.collect(components);
        components.set_container(&self.items);
        self.loot_table.collect(components);
    }

    /// `RandomizableContainerBlockEntity.applyImplicitComponents` (`RandomizableContainerBlockEntity.java:97-104`).
    pub(super) fn apply_implicit_components(
        &mut self,
        components: &mut ComponentMapReader<'_>,
    ) -> bbb_server_types::Result<()> {
        self.base.apply_components(components)?;
        components.copy_container_into(&mut self.items)?;
        self.loot_table.apply_components(components)
    }

    pub(super) const fn loot_table(&self) -> &ContainerLootTable {
        &self.loot_table
    }

    pub(super) fn randomizable_parts_mut(
        &mut self,
    ) -> (&mut ContainerLootTable, &mut [RuntimeItemStack]) {
        (&mut self.loot_table, &mut self.items)
    }

    #[must_use]
    pub fn items(&self) -> Vec<RuntimeItemStack> {
        self.items.to_vec()
    }

    pub fn original_items(&self) -> &[RuntimeItemStack] {
        &self.items
    }

    pub fn custom_name(&self) -> Option<&NbtSummaryValue> {
        self.base.custom_name()
    }

    pub fn can_open(
        &self,
        is_spectator: bool,
        key: &RuntimeItemStack,
        registries: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
    ) -> bbb_server_types::Result<bool> {
        if self.loot_table.is_set() && is_spectator {
            return Ok(false);
        }
        Ok(self.base.can_unlock(is_spectator, key, registries))
    }

    /// `CrafterBlockEntity.java:88-113`: fill empty/enabled slots before increasing an earlier stack.
    pub fn can_place_item(
        &self,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        slot: usize,
    ) -> Result<bool> {
        if self.is_slot_disabled(slot) {
            return Ok(false);
        }
        let Some(current) = self.items.get(slot) else {
            return Ok(false);
        };
        if current.count() >= current.max_stack_size()? {
            return Ok(false);
        }
        if slot_is_empty(current) {
            return Ok(true);
        }
        for (index, other) in self.items.iter().enumerate().skip(slot + 1) {
            if !self.is_slot_disabled(index)
                && (slot_is_empty(other)
                    || other.count() < current.count()
                        && other.same_item_same_effective_components(current, resolver)?)
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// `CrafterBlockEntity.getRedstoneSignal` (`CrafterBlockEntity.java:251-262`):
    /// a count of slots rather than a fill fraction, and a disabled slot counts
    /// as a taken one.
    #[must_use]
    pub fn redstone_signal(&self) -> i32 {
        self.items
            .iter()
            .zip(self.disabled)
            .filter(|(item, disabled)| !slot_is_empty(item) || *disabled)
            .count() as i32
    }

    pub fn items_mut(&mut self) -> &mut [RuntimeItemStack] {
        &mut self.items
    }

    /// `CrafterBlockEntity.setSlotState` (`CrafterBlockEntity.java:76-81`).
    pub fn set_slot_disabled(&mut self, slot: i32, disabled: bool) -> bool {
        if !self.slot_can_be_disabled(slot) {
            return false;
        }
        self.disabled[slot as usize] = disabled;
        true
    }

    #[must_use]
    pub const fn crafting_ticks_remaining(&self) -> i32 {
        self.crafting_ticks_remaining
    }

    /// `CrafterBlockEntity.java:247-248`.
    pub fn set_crafting_ticks_remaining(&mut self, ticks: i32) {
        self.crafting_ticks_remaining = ticks;
    }

    /// `CrafterBlockEntity.java:237-243`.
    #[must_use]
    pub fn server_tick(&mut self) -> bool {
        let remaining = self.crafting_ticks_remaining.wrapping_sub(1);
        if remaining >= 0 {
            self.crafting_ticks_remaining = remaining;
            remaining == 0
        } else {
            false
        }
    }

    #[must_use]
    pub const fn triggered(&self) -> i32 {
        self.triggered
    }

    /// `CrafterBlockEntity.java:228-229`.
    pub fn set_triggered(&mut self, triggered: bool) {
        self.triggered = i32::from(triggered);
    }

    #[must_use]
    pub fn is_slot_disabled(&self, slot: usize) -> bool {
        self.disabled.get(slot).copied().unwrap_or(false)
    }

    /// `CrafterBlockEntity.setSlotState(slot, true)` (`:76-81`) for a disabled
    /// slot, which `slotCanBeDisabled` admits only while it is empty.
    pub(super) fn enable_disabled_slot(&mut self, slot: usize) -> bool {
        let Ok(index) = i32::try_from(slot) else {
            return false;
        };
        if !self.is_slot_disabled(slot) || !self.slot_can_be_disabled(index) {
            return false;
        }
        self.disabled[slot] = false;
        true
    }

    /// `CrafterBlockEntity.saveAdditional` (`:139-148`) over
    /// `BaseContainerBlockEntity.saveAdditional` (`BaseContainerBlockEntity.java:45-49`).
    pub(super) fn save_additional(
        &self,
        save: &mut BlockEntityItemSave<'_>,
    ) -> Result<Vec<NbtSummaryEntry>> {
        let mut entries = Vec::new();
        self.base.save(&mut entries);
        entries.push(entry(
            TAG_CRAFTING_TICKS_REMAINING,
            NbtSummaryValue::Int(self.crafting_ticks_remaining),
        ));
        if !self.loot_table.try_save(&mut entries) {
            entries.push(entry(TAG_ITEMS, saved_items(&self.items, save)?));
        }
        entries.push(entry(
            TAG_DISABLED_SLOTS,
            NbtSummaryValue::IntArray(self.saved_disabled_slots()),
        ));
        entries.push(entry(TAG_TRIGGERED, NbtSummaryValue::Int(self.triggered)));
        Ok(entries)
    }

    /// `CrafterBlockEntity.loadAdditional` (`:116-136`).
    pub(super) fn load_additional(
        &mut self,
        record: &[NbtSummaryEntry],
        load: &mut BlockEntityItemLoad<'_>,
    ) -> Result<()> {
        self.base = ContainerNameAndLock::load(record);
        self.crafting_ticks_remaining = int_or(record, TAG_CRAFTING_TICKS_REMAINING, 0);
        self.items = vec![RuntimeItemStack::empty(); CRAFTER_CONTAINER_SIZE];
        self.loot_table = ContainerLootTable::try_load(record);
        if !self.loot_table.is_set() {
            load_items(unique_field(record, TAG_ITEMS), &mut self.items, load)?;
        }
        self.disabled = [false; CRAFTER_CONTAINER_SIZE];
        if let Some(NbtSummaryValue::IntArray(slots)) = unique_field(record, TAG_DISABLED_SLOTS) {
            for slot in slots {
                if self.slot_can_be_disabled(*slot) {
                    self.disabled[*slot as usize] = true;
                }
            }
        }
        self.triggered = int_or(record, TAG_TRIGGERED, 0);
        Ok(())
    }

    /// `CrafterBlockEntity.addDisabledSlots` (`:212-222`).
    fn saved_disabled_slots(&self) -> Vec<i32> {
        self.disabled
            .iter()
            .enumerate()
            .filter(|(_, disabled)| **disabled)
            .map(|(slot, _)| slot as i32)
            .collect()
    }

    /// `CrafterBlockEntity.slotCanBeDisabled` (`:264-266`).
    fn slot_can_be_disabled(&self, slot: i32) -> bool {
        let Ok(index) = usize::try_from(slot) else {
            return false;
        };
        index < CRAFTER_CONTAINER_SIZE && slot_is_empty(&self.items[index])
    }
}

fn slot_is_empty(item: &RuntimeItemStack) -> bool {
    item.is_empty()
}

/// `Containers.dropContents` over the nine crafter slots.
#[must_use]
pub fn drop_contents(
    crafter: &CrafterBlockEntity,
) -> Vec<bbb_server_types::item_stack::RuntimeItemStack> {
    crafter
        .original_items()
        .iter()
        .map(bbb_server_types::item_stack::RuntimeItemStack::clone)
        .collect()
}

#[cfg(test)]
mod tests {

    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::VANILLA_BLOCK_CRAFTER_ID;
    use bbb_protocol::item_types::VANILLA_ITEM_STONE_ID;
    use bbb_protocol::packets::{BlockPos, ItemStackSummary};
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

    use crate::block_entity::{
        new_block_entity, unique_field, BlockEntityPayload, BlockEntityRemovalDrops,
    };

    use super::*;

    fn pos() -> BlockPos {
        BlockPos { x: 3, y: 4, z: 5 }
    }

    fn default_state() -> i32 {
        vanilla_block_default_state_id(VANILLA_BLOCK_CRAFTER_ID)
            .expect("minecraft:crafter has a default state")
    }

    fn stone(count: i32) -> RuntimeItemStack {
        RuntimeItemStack::from_summary(ItemStackSummary {
            item_id: Some(VANILLA_ITEM_STONE_ID),
            count,
            component_patch: Default::default(),
        })
    }

    fn replace_additional(record: &mut Vec<NbtSummaryEntry>, extra: Vec<NbtSummaryEntry>) {
        const OWNED: &[&str] = &[
            TAG_ITEMS,
            "LootTable",
            "LootTableSeed",
            TAG_CRAFTING_TICKS_REMAINING,
            TAG_DISABLED_SLOTS,
            TAG_TRIGGERED,
            "lock",
            "CustomName",
        ];
        record.retain(|entry| !OWNED.iter().any(|name| entry.name == *name));
        record.extend(extra);
    }

    #[test]
    fn crafter_initial_triggered_comes_from_state_and_loading_overrides_it() {
        let state = crate::fluid::with_property(default_state(), "triggered", "true")
            .expect("crafter triggered state");
        let mut built = new_block_entity(pos(), state).expect("a crafter builds an object");
        assert!(crate::block_entity::has_ticker(&built));
        assert_eq!(built.crafter().unwrap().triggered(), 1);
        built.crafter_mut().unwrap().set_triggered(false);
        let record = built
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        let loaded = crate::block_entity::load_test_fixture(pos(), state, &record)
            .expect("fixture codec succeeds")
            .expect("the record names a crafter");
        assert_eq!(loaded.crafter().unwrap().triggered(), 0);
    }

    #[test]
    fn crafter_save_load_round_trips_items_ticks_disabled_and_triggered() {
        let mut crafter = CrafterBlockEntity::default();
        crafter.items[0] = stone(2);
        crafter.items[8] = stone(1);
        crafter.set_crafting_ticks_remaining(6);
        crafter.disabled[4] = true;
        crafter.set_triggered(true);
        let mut record = new_block_entity(pos(), default_state())
            .expect("a crafter builds an object")
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        replace_additional(
            &mut record,
            crafter
                .save_additional(&mut BlockEntityItemSave::for_test())
                .unwrap(),
        );
        let loaded = crate::block_entity::load_test_fixture(pos(), default_state(), &record)
            .expect("fixture codec succeeds")
            .expect("the record names a crafter");
        let BlockEntityPayload::Crafter(loaded) = loaded.payload() else {
            panic!("payload is a crafter");
        };
        assert_eq!(loaded.items()[0].item_id(), Some(VANILLA_ITEM_STONE_ID));
        assert_eq!(loaded.items()[0].count(), 2);
        assert!(slot_is_empty(&loaded.items()[1]));
        assert_eq!(loaded.items()[8].count(), 1);
        assert_eq!(loaded.crafting_ticks_remaining(), 6);
        assert!(loaded.is_slot_disabled(4));
        assert!(!loaded.is_slot_disabled(0));
        assert_eq!(loaded.triggered(), 1);
        let again = crate::block_entity::load_test_fixture(
            pos(),
            default_state(),
            &crate::block_entity::load_test_fixture(pos(), default_state(), &record)
                .expect("fixture codec succeeds")
                .expect("reload")
                .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
                .expect("fixture codec succeeds"),
        )
        .expect("fixture codec succeeds")
        .expect("the round-trip still names a crafter");
        let BlockEntityPayload::Crafter(again) = again.payload() else {
            panic!("payload is a crafter");
        };
        assert_eq!(again.crafting_ticks_remaining(), 6);
        assert!(again.is_slot_disabled(4));
        assert_eq!(again.triggered(), 1);
        let reloaded = crate::block_entity::load_test_fixture(pos(), default_state(), &record)
            .expect("fixture codec succeeds")
            .expect("drops");
        let drops = reloaded.pre_remove_side_effects(reloaded.block_state());
        let BlockEntityRemovalDrops::ContainerSlots(slots) = drops else {
            panic!("a crafter is a container");
        };
        assert_eq!(slots.len(), 9);
        assert_eq!(slots[0].summary(), stone(2).summary());
        assert!(slot_is_empty(&slots[1]));
        assert_eq!(slots[8].summary(), stone(1).summary());

        let mut resumed = crate::block_entity::load_test_fixture(pos(), default_state(), &record)
            .expect("fixture codec succeeds")
            .expect("the record names a crafter");
        let crafter = resumed.crafter_mut().unwrap();
        for remaining in (0..6).rev() {
            assert_eq!(crafter.server_tick(), remaining == 0);
            assert_eq!(crafter.crafting_ticks_remaining(), remaining);
        }
        assert!(!crafter.server_tick());
        assert_eq!(crafter.crafting_ticks_remaining(), 0);
        assert_eq!(crafter.items()[0], stone(2));
        assert_eq!(crafter.triggered(), 1);
        assert!(crafter.is_slot_disabled(4));
    }

    #[test]
    fn a_packed_loot_table_skips_items() {
        let mut crafter = CrafterBlockEntity::default();
        crafter.items[0] = stone(1);
        crafter.loot_table.set(
            &NbtSummaryValue::string("minecraft:chests/simple_dungeon"),
            99,
        );
        let mut record = new_block_entity(pos(), default_state())
            .expect("a crafter builds an object")
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        replace_additional(
            &mut record,
            crafter
                .save_additional(&mut BlockEntityItemSave::for_test())
                .unwrap(),
        );
        assert!(unique_field(&record, TAG_ITEMS).is_none());
        assert_eq!(
            unique_field(&record, "LootTableSeed"),
            Some(&NbtSummaryValue::Long(99))
        );
        let loaded = crate::block_entity::load_test_fixture(pos(), default_state(), &record)
            .expect("fixture codec succeeds")
            .expect("the record names a crafter");
        let BlockEntityPayload::Crafter(payload) = loaded.payload() else {
            panic!("payload is a crafter");
        };
        assert!(payload.items().iter().all(slot_is_empty));
        let saved = loaded
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        assert_eq!(
            unique_field(&saved, "LootTable"),
            Some(&NbtSummaryValue::string("minecraft:chests/simple_dungeon"))
        );
        assert!(unique_field(&saved, TAG_ITEMS).is_none());
    }
}
