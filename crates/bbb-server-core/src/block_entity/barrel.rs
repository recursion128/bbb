//! `BarrelBlockEntity` (`BarrelBlockEntity.java:25-141`).
//!
//! The 27-slot `RandomizableContainerBlockEntity` body is the same schema a
//! chest already carries — `Items`, lock, custom name, loot table, opener
//! count — so this type holds a [`super::chest::ChestBlockEntity`] rather than
//! restating it. What is not a chest is everything around that body:
//! `BarrelBlock.OPEN` instead of a lid block event, barrel sounds, and no
//! `triggerEvent` override.

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::Result;

use super::chest::ChestBlockEntity;
use super::components::{ComponentMapBuilder, ComponentMapReader};
use super::container_fields::ContainerLootTable;
use super::{BlockEntityItemLoad, BlockEntityItemSave};
use bbb_server_types::item_stack::RuntimeItemStack;

/// `BarrelBlockEntity` (`BarrelBlockEntity.java:25-28`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BarrelBlockEntity {
    inner: ChestBlockEntity,
}

impl BarrelBlockEntity {
    #[must_use]
    pub fn items(&self) -> Vec<RuntimeItemStack> {
        self.inner.items()
    }

    pub fn original_items(&self) -> &[RuntimeItemStack] {
        self.inner.original_items()
    }

    pub fn items_mut(&mut self) -> &mut [RuntimeItemStack] {
        self.inner.items_mut()
    }

    pub fn set_items(&mut self, items: Vec<RuntimeItemStack>) -> bool {
        self.inner.set_items(items)
    }

    #[must_use]
    pub const fn open_count(&self) -> i32 {
        self.inner.open_count()
    }

    /// `ContainerOpenersCounter.recheckOpeners` (`:66-94`) count write.
    pub fn set_open_count(&mut self, count: i32) {
        self.inner.set_open_count(count);
    }

    pub fn redstone_signal(barrel: &Self) -> bbb_server_types::Result<i32> {
        ChestBlockEntity::redstone_signal(&barrel.inner)
    }

    #[must_use]
    pub fn custom_name(&self) -> Option<&NbtSummaryValue> {
        self.inner.custom_name()
    }

    pub fn can_open(
        &self,
        is_spectator: bool,
        key: &RuntimeItemStack,
        registries: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
    ) -> bbb_server_types::Result<bool> {
        self.inner.can_open(is_spectator, key, registries)
    }

    pub(crate) fn max_interaction_range(&self) -> f64 {
        self.inner.max_interaction_range()
    }

    pub(crate) fn set_max_interaction_range(&mut self, range: f64) {
        self.inner.set_max_interaction_range(range);
    }

    pub(super) fn collect_implicit_components(&self, components: &mut ComponentMapBuilder<'_>) {
        self.inner.collect_implicit_components(components);
    }

    pub(super) fn apply_implicit_components(
        &mut self,
        components: &mut ComponentMapReader<'_>,
    ) -> bbb_server_types::Result<()> {
        self.inner.apply_implicit_components(components)
    }

    pub(super) const fn loot_table(&self) -> &ContainerLootTable {
        self.inner.loot_table()
    }

    pub(super) fn randomizable_parts_mut(
        &mut self,
    ) -> (&mut ContainerLootTable, &mut [RuntimeItemStack]) {
        self.inner.randomizable_parts_mut()
    }

    pub(super) fn save_additional(
        &self,
        save: &mut BlockEntityItemSave<'_>,
    ) -> Result<Vec<NbtSummaryEntry>> {
        self.inner.save_additional(save)
    }

    pub(super) fn load_additional(
        &mut self,
        record: &[NbtSummaryEntry],
        load: &mut BlockEntityItemLoad<'_>,
    ) -> Result<()> {
        self.inner.load_additional(record, load)
    }
}

/// `Containers.dropContents` over a barrel.
pub fn drop_contents(
    barrel: &BarrelBlockEntity,
) -> Vec<bbb_server_types::item_stack::RuntimeItemStack> {
    barrel
        .original_items()
        .iter()
        .map(bbb_server_types::item_stack::RuntimeItemStack::clone)
        .collect()
}
