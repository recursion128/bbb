//! `DispenserBlockEntity` (`DispenserBlockEntity.java:16-109`) and
//! `DropperBlockEntity` (`DropperBlockEntity.java:7-18`), which only replaces
//! the default name.
//!
//! Nine slots, the `Items` schema, and `getRandomSlot`. Special `DispenseItemBehavior`
//! registrations stay outside this slice; dropper-into-container and the
//! default item eject are the production path.

use crate::random::GameRandom;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::Result;

use super::chest::{load_items, saved_items, TAG_ITEMS};
use super::components::{ComponentMapBuilder, ComponentMapReader};
use super::container_fields::{ContainerLootTable, ContainerNameAndLock};
use super::{entry, unique_field, BlockEntityItemLoad, BlockEntityItemSave};
use crate::inventory::redstone_signal_from_container;
use bbb_server_types::item_stack::RuntimeItemStack;

/// `DispenserBlockEntity.CONTAINER_SIZE` (`DispenserBlockEntity.java:17`).
pub const DISPENSER_CONTAINER_SIZE: usize = 9;

/// `DispenserBlockEntity` (`DispenserBlockEntity.java:16-19`).
#[derive(Debug, Clone, PartialEq)]
pub struct DispenserBlockEntity {
    items: Vec<RuntimeItemStack>,
    base: ContainerNameAndLock,
    loot_table: ContainerLootTable,
}

impl Default for DispenserBlockEntity {
    fn default() -> Self {
        Self {
            items: vec![RuntimeItemStack::empty(); DISPENSER_CONTAINER_SIZE],
            base: ContainerNameAndLock::default(),
            loot_table: ContainerLootTable::default(),
        }
    }
}

impl DispenserBlockEntity {
    #[must_use]
    pub fn items(&self) -> Vec<RuntimeItemStack> {
        self.items.to_vec()
    }

    pub fn original_items(&self) -> &[RuntimeItemStack] {
        &self.items
    }

    pub fn items_mut(&mut self) -> &mut [RuntimeItemStack] {
        &mut self.items
    }

    /// `RandomizableContainer.setLootTable` (`RandomizableContainer.java:29-31`).
    pub fn set_loot_table(&mut self, loot_table: NbtSummaryValue, seed: i64) {
        self.loot_table.set(&loot_table, seed);
    }

    /// `RandomizableContainerBlockEntity.collectImplicitComponents`
    /// (`RandomizableContainerBlockEntity.java:107-112`) over
    /// `BaseContainerBlockEntity`'s (`BaseContainerBlockEntity.java:157-165`).
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

    pub fn set_items(&mut self, items: Vec<RuntimeItemStack>) -> bool {
        if items.len() != self.items.len() {
            return false;
        }
        self.items = items.into_iter().collect();
        true
    }

    pub fn redstone_signal(dispenser: &Self) -> bbb_server_types::Result<i32> {
        redstone_signal_from_container(&dispenser.items())
    }

    #[must_use]
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

    /// `DispenserBlockEntity.getRandomSlot` (`DispenserBlockEntity.java:34-46`).
    #[must_use]
    pub fn random_slot(&self, random: &GameRandom) -> Option<usize> {
        random_filled_slot(&self.items(), random)
    }

    pub(super) fn save_additional(
        &self,
        save: &mut BlockEntityItemSave<'_>,
    ) -> Result<Vec<NbtSummaryEntry>> {
        let mut entries = Vec::new();
        self.base.save(&mut entries);
        if self.loot_table.try_save(&mut entries) {
            return Ok(entries);
        }
        entries.push(entry(TAG_ITEMS, saved_items(&self.items, save)?));
        Ok(entries)
    }

    pub(super) fn load_additional(
        &mut self,
        record: &[NbtSummaryEntry],
        load: &mut BlockEntityItemLoad<'_>,
    ) -> Result<()> {
        self.base = ContainerNameAndLock::load(record);
        self.loot_table = ContainerLootTable::try_load(record);
        self.items = vec![RuntimeItemStack::empty(); DISPENSER_CONTAINER_SIZE];
        if self.loot_table.is_set() {
            return Ok(());
        }
        load_items(unique_field(record, TAG_ITEMS), &mut self.items, load)?;
        Ok(())
    }
}

pub(crate) fn random_filled_slot(items: &[RuntimeItemStack], random: &GameRandom) -> Option<usize> {
    let mut chosen = None;
    let mut odds = 1i32;
    for (slot, item) in items.iter().enumerate() {
        if item.is_empty() {
            continue;
        }
        if random.next_int(odds) == 0 {
            chosen = Some(slot);
        }
        odds += 1;
    }
    chosen
}

/// `Containers.dropContents` over a dispenser or dropper.
pub fn drop_contents(
    dispenser: &DispenserBlockEntity,
) -> Vec<bbb_server_types::item_stack::RuntimeItemStack> {
    dispenser
        .original_items()
        .iter()
        .map(bbb_server_types::item_stack::RuntimeItemStack::clone)
        .collect()
}
