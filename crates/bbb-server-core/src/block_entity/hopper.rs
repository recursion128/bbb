//! Hopper state and container slot merging.

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::geometry::Aabb;
use bbb_server_types::Result;

use super::chest::{load_items, saved_items, TAG_ITEMS};
use super::components::{ComponentMapBuilder, ComponentMapReader};
use super::container_fields::{ContainerLootTable, ContainerNameAndLock};
use super::value_input::int_or;
use super::{entry, unique_field, BlockEntityItemLoad, BlockEntityItemSave};
use crate::inventory::redstone_signal_from_container;
use bbb_server_types::item_stack::RuntimeItemStack;

/// `HopperBlockEntity.HOPPER_CONTAINER_SIZE` (`HopperBlockEntity.java:33`).
pub const HOPPER_CONTAINER_SIZE: usize = 5;
/// `HopperBlockEntity.MOVE_ITEM_SPEED` (`HopperBlockEntity.java:32`).
pub const MOVE_ITEM_SPEED: i32 = 8;
/// `Hopper.SUCK_AABB` (`Hopper.java:8`), the block-local box
/// `Block.column(16.0, 11.0, 32.0)` builds (`Block.java:181-189`).
pub(crate) const SUCK_AABB: Aabb = Aabb {
    min_x: 0.0,
    min_y: 11.0 / 16.0,
    min_z: 0.0,
    max_x: 1.0,
    max_y: 2.0,
    max_z: 1.0,
};

/// `HopperBlockEntity.NO_COOLDOWN_TIME` (`HopperBlockEntity.java:35`).
const NO_COOLDOWN_TIME: i32 = -1;
/// The save field `saveAdditional` writes (`HopperBlockEntity.java:65`).
const TAG_TRANSFER_COOLDOWN: &str = "TransferCooldown";

/// `HopperBlockEntity` (`HopperBlockEntity.java:31-40`).
#[derive(Debug, Clone, PartialEq)]
pub struct HopperBlockEntity {
    items: Vec<RuntimeItemStack>,
    base: ContainerNameAndLock,
    loot_table: ContainerLootTable,
    cooldown_time: i32,
    ticked_game_time: i64,
}

impl Default for HopperBlockEntity {
    /// `new HopperBlockEntity(pos, state)` (`:42-45`): five empty slots and
    /// cooldown `-1`.
    fn default() -> Self {
        Self {
            items: vec![RuntimeItemStack::empty(); HOPPER_CONTAINER_SIZE],
            base: ContainerNameAndLock::default(),
            loot_table: ContainerLootTable::default(),
            cooldown_time: NO_COOLDOWN_TIME,
            ticked_game_time: 0,
        }
    }
}

impl HopperBlockEntity {
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

    /// `HopperBlockEntity.getCooldown` as the saved `TransferCooldown`.
    #[must_use]
    pub const fn cooldown_time(&self) -> i32 {
        self.cooldown_time
    }

    /// `HopperBlockEntity.isOnCooldown` (`:430-432`).
    #[must_use]
    pub const fn is_on_cooldown(&self) -> bool {
        self.cooldown_time > 0
    }

    /// `HopperBlockEntity.setCooldown` (`:426-428`).
    pub fn set_cooldown(&mut self, time: i32) {
        self.cooldown_time = time;
    }

    /// `AbstractContainerMenu.getRedstoneSignalFromContainer` over five slots.
    pub fn redstone_signal(hopper: &Self) -> bbb_server_types::Result<i32> {
        redstone_signal_from_container(&hopper.items)
    }

    /// `HopperBlockEntity.saveAdditional` (`:59-66`).
    pub fn save_additional(
        &self,
        save: &mut BlockEntityItemSave<'_>,
    ) -> Result<Vec<NbtSummaryEntry>> {
        let mut entries = Vec::new();
        self.base.save(&mut entries);
        if !self.loot_table.try_save(&mut entries) {
            entries.push(entry(TAG_ITEMS, saved_items(&self.items, save)?));
        }
        entries.push(entry(
            TAG_TRANSFER_COOLDOWN,
            NbtSummaryValue::Int(self.cooldown_time),
        ));
        Ok(entries)
    }

    /// `HopperBlockEntity.loadAdditional` (`:48-56`).
    pub fn load_additional(
        &mut self,
        record: &[NbtSummaryEntry],
        load: &mut BlockEntityItemLoad<'_>,
    ) -> Result<()> {
        self.base = ContainerNameAndLock::load(record);
        self.items = vec![RuntimeItemStack::empty(); HOPPER_CONTAINER_SIZE];
        self.loot_table = ContainerLootTable::try_load(record);
        if !self.loot_table.is_set() {
            load_items(unique_field(record, TAG_ITEMS), &mut self.items, load)?;
        }
        self.cooldown_time = int_or(record, TAG_TRANSFER_COOLDOWN, NO_COOLDOWN_TIME);
        Ok(())
    }

    pub fn begin_transfer_tick(&mut self, game_time: i64) -> bool {
        self.cooldown_time = self.cooldown_time.wrapping_sub(1);
        self.ticked_game_time = game_time;
        if self.is_on_cooldown() {
            return false;
        }
        self.set_cooldown(0);
        true
    }

    pub const fn ticked_game_time(&self) -> i64 {
        self.ticked_game_time
    }
}

pub(crate) fn merge_item(
    current: &mut RuntimeItemStack,
    item: &mut RuntimeItemStack,
    resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
) -> Result<bool> {
    if current.is_empty() {
        *current = std::mem::replace(item, RuntimeItemStack::empty());
        return Ok(true);
    }
    if !current.same_item_same_effective_components(item, resolver)?
        || current.count() > current.max_stack_size()?
    {
        return Ok(false);
    }
    let count = item.count().min(item.max_stack_size()? - current.count());
    if count <= 0 {
        return Ok(false);
    }
    current.set_count(current.count() + count);
    item.set_count(item.count() - count);
    Ok(true)
}

/// Contents `preRemoveSideEffects` drops.
#[must_use]
pub fn drop_contents(
    hopper: &HopperBlockEntity,
) -> Vec<bbb_server_types::item_stack::RuntimeItemStack> {
    hopper
        .original_items()
        .iter()
        .map(bbb_server_types::item_stack::RuntimeItemStack::clone)
        .collect()
}
