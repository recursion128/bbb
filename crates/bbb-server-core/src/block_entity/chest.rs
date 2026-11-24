//! `ChestBlockEntity` (`ChestBlockEntity.java:28-199`) and the three classes
//! above it that supply most of its body.
//!
//! * `RandomizableContainerBlockEntity`
//!   (`RandomizableContainerBlockEntity.java:19-119`) — the loot-table pair
//!   and the `createMenu` that unpacks it.
//! * `BaseContainerBlockEntity` (`BaseContainerBlockEntity.java:29-172`) —
//!   the lock, the custom name, and every `Container` method a block entity
//!   answers over `getItems()`.
//! * `Container` (`Container.java:19-123`) itself, whose `getMaxStackSize` is
//!   the literal 99 (`:34-36`) that [`crate::inventory`] already carries.
//!
//! # What a chest is here
//!
//! The slots are the whole of it: `items` (`ChestBlockEntity.java:31`) is 27
//! stacks, and the menu that addresses them lives in [`crate::inventory`].
//! Everything else this file holds is state a save carries that must survive a
//! round trip — the lock, the custom name and the loot-table pair — kept as
//! the record's own values rather than decoded, because nothing here reads
//! them beyond deciding whether the chest opens.
//!
//! # The opener counter
//!
//! `ContainerOpenersCounter` (`ContainerOpenersCounter.java:15-102`) is the
//! part of a chest a client can see without opening one: the lid, the sound
//! and the block event that drives both. Its `openCount` is runtime state, so
//! no save carries it — `recheckOpeners` (`:66-94`) rebuilds it from the
//! players who have the menu open. Increment and decrement live on
//! [`ChestBlockEntity`]; `ChestBlock.tick` (`ChestBlock.java:384-389`) runs
//! the five-tick `recheckOpeners` sweep.

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::item_stack::RuntimeItemStack;

use super::components::{ComponentMapBuilder, ComponentMapReader};
use super::container_fields::{ContainerLootTable, ContainerNameAndLock};
use super::{entry, unique_field, BlockEntityItemLoad, BlockEntityItemSave};
use crate::inventory::{redstone_signal_from_container, CHEST_CONTAINER_SIZE};

/// `ContainerHelper.TAG_ITEMS` (`ContainerHelper.java:11`).
pub(super) const TAG_ITEMS: &str = "Items";
/// `ItemStackWithSlot.CODEC`'s slot field (`ItemStackWithSlot.java:12`), an
/// `ExtraCodecs.UNSIGNED_BYTE` and therefore a byte tag on disk.
pub(super) const TAG_SLOT: &str = "Slot";

/// `ChestBlockEntity` (`ChestBlockEntity.java:28-31`).
#[derive(Debug, Clone, PartialEq)]
pub struct ChestBlockEntity {
    /// `ChestBlockEntity.items` (`:31`), 27 stacks.
    items: Vec<RuntimeItemStack>,
    base: ContainerNameAndLock,
    loot_table: ContainerLootTable,
    /// `ContainerOpenersCounter.openCount`
    /// (`ContainerOpenersCounter.java:17`), which no save carries.
    open_count: i32,
    max_interaction_range: f64,
}

impl Default for ChestBlockEntity {
    /// `new ChestBlockEntity(pos, state)` (`ChestBlockEntity.java:68-70`): 27
    /// empty stacks and nothing else.
    fn default() -> Self {
        Self {
            items: vec![RuntimeItemStack::empty(); CHEST_CONTAINER_SIZE],
            base: ContainerNameAndLock::default(),
            loot_table: ContainerLootTable::default(),
            open_count: 0,
            max_interaction_range: 0.0,
        }
    }
}

impl ChestBlockEntity {
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

    /// `BaseContainerBlockEntity.getItem` (`:100-102`) over the whole
    /// container at once, which is what the menu addresses.
    #[must_use]
    pub fn items(&self) -> Vec<RuntimeItemStack> {
        self.items.to_vec()
    }

    pub fn original_items(&self) -> &[RuntimeItemStack] {
        &self.items
    }

    /// Slot storage the hopper transfer walk mutates in place.
    pub fn items_mut(&mut self) -> &mut [RuntimeItemStack] {
        &mut self.items
    }

    /// `BaseContainerBlockEntity.setItem` (`:120-124`) over the whole
    /// container. A list of the wrong length is refused: `getContainerSize`
    /// (`ChestBlockEntity.java:73-75`) is fixed at 27 and nothing may resize
    /// it.
    pub fn set_items(&mut self, items: Vec<RuntimeItemStack>) -> bool {
        if items.len() != self.items.len() {
            return false;
        }
        self.items = items.into_iter().collect();
        true
    }

    /// `ContainerOpenersCounter.getOpenerCount` (`:96-98`), reached through
    /// `ChestBlockEntity.getOpenCount` (`:166-176`).
    #[must_use]
    pub const fn open_count(&self) -> i32 {
        self.open_count
    }

    /// `ContainerOpenersCounter.recheckOpeners` (`:66-94`) count write.
    pub fn set_open_count(&mut self, count: i32) {
        self.open_count = count;
    }

    pub(crate) fn max_interaction_range(&self) -> f64 {
        self.max_interaction_range
    }

    pub(crate) fn set_max_interaction_range(&mut self, range: f64) {
        self.max_interaction_range = range;
    }

    /// `AbstractContainerMenu.getRedstoneSignalFromContainer`
    /// (`AbstractContainerMenu.java:751-767`) over this chest's 27 slots.
    pub fn redstone_signal(chest: &Self) -> bbb_server_types::Result<i32> {
        redstone_signal_from_container(&chest.items())
    }

    #[must_use]
    pub fn custom_name(&self) -> Option<&NbtSummaryValue> {
        self.base.custom_name()
    }

    pub(crate) fn is_locked(&self) -> bool {
        self.base.is_locked()
    }

    /// `RandomizableContainerBlockEntity.canOpen` (`RandomizableContainerBlockEntity.java:78-80`).
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

    /// `ContainerOpenersCounter.incrementOpeners` (`:28-38`) as
    /// `ChestBlockEntity.startOpen` (`:130-137`) reaches it.
    ///
    /// The `GameEvent.CONTAINER_OPEN` broadcast (`:32`) and the five-tick
    /// `scheduleRecheck` (`:33`) are the caller's; what is left here is the
    /// count and which of the two callbacks the change fires.
    pub fn start_open(&mut self) -> OpenersChange {
        let previous = self.open_count;
        self.open_count += 1;
        OpenersChange {
            previous,
            current: self.open_count,
            // `:30-31`: `onOpen` fires only on the transition out of zero.
            crossed: previous == 0,
        }
    }

    /// `ContainerOpenersCounter.decrementOpeners` (`:40-49`) as
    /// `ChestBlockEntity.stopOpen` (`:140-144`) reaches it.
    pub fn stop_open(&mut self) -> OpenersChange {
        let previous = self.open_count;
        self.open_count -= 1;
        OpenersChange {
            previous,
            current: self.open_count,
            // `:42-46`: `onClose` fires only on the transition back to zero.
            crossed: self.open_count == 0,
        }
    }

    /// `ChestBlockEntity.triggerEvent` (`:120-127`): event 1 drives the lid,
    /// and every `BlockEntity.triggerEvent` answers whether it handled the
    /// event (`BlockEntity.java:239-241`).
    ///
    /// The lid controller itself (`ChestLidController.java:5-27`) is client
    /// state; the server's whole share of the event is answering true so
    /// `ServerLevel.doBlockEvent` (`ServerLevel.java:1268-1271`) broadcasts it.
    #[must_use]
    pub fn trigger_event(&mut self, param_a: i32) -> bool {
        let _ = param_a;
        true
    }

    /// `ChestBlockEntity.saveAdditional` (`:92-97`) over
    /// `BaseContainerBlockEntity.saveAdditional` (`:45-49`), in that order:
    /// the lock, the custom name, and then either the loot-table pair or the
    /// items — never both, because `trySaveLootTable`
    /// (`RandomizableContainer.java:57-70`) answers true once it has stored
    /// one.
    pub(super) fn save_additional(
        &self,
        save: &mut BlockEntityItemSave<'_>,
    ) -> bbb_server_types::Result<Vec<NbtSummaryEntry>> {
        let mut entries = Vec::new();
        self.base.save(&mut entries);
        if self.loot_table.try_save(&mut entries) {
            return Ok(entries);
        }
        entries.push(entry(TAG_ITEMS, saved_items(&self.items, save)?));
        Ok(entries)
    }

    /// `ChestBlockEntity.loadAdditional` (`:83-89`) over
    /// `BaseContainerBlockEntity.loadAdditional` (`:38-42`).
    pub(super) fn load_additional(
        &mut self,
        record: &[NbtSummaryEntry],
        load: &mut BlockEntityItemLoad<'_>,
    ) -> bbb_server_types::Result<()> {
        self.base = ContainerNameAndLock::load(record);
        // `RandomizableContainer.tryLoadLootTable` (`:50-55`) reads both
        // fields and answers whether a table was there; only when it was not
        // does `ContainerHelper.loadAllItems` run (`ChestBlockEntity.java:86-88`).
        self.loot_table = ContainerLootTable::try_load(record);
        // `:85`: the list is rebuilt empty before either branch runs, so a
        // record that carries neither field leaves an empty chest.
        self.items = vec![RuntimeItemStack::empty(); CHEST_CONTAINER_SIZE];
        if self.loot_table.is_set() {
            return Ok(());
        }
        load_items(unique_field(record, TAG_ITEMS), &mut self.items, load)?;
        Ok(())
    }
}

/// What one `incrementOpeners`/`decrementOpeners` did, so the caller can run
/// the halves that need a level: the sound `onOpen`/`onClose` plays
/// (`ChestBlockEntity.java:34-45`) and the block event `openerCountChanged`
/// raises (`:48-50`, `:195-198`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenersChange {
    pub previous: i32,
    pub current: i32,
    /// Whether the count crossed zero, which is the gate on `onOpen`/`onClose`.
    pub crossed: bool,
}

/// `ContainerHelper.saveAllItems` (`ContainerHelper.java:25-38`): one entry per
/// non-empty stack, each `ItemStackWithSlot.CODEC`
/// (`ItemStackWithSlot.java:9-14`) — the slot byte beside the stack's own
/// `ItemStack.MAP_CODEC` fields, inlined rather than nested.
///
/// `ValueOutput.TypedOutputList.add` (`TagValueOutput.java:225-235`) reports an
/// element the codec refuses and appends only the partial value an error
/// carried, so a stack this publication cannot spell leaves the list while
/// `save` names the slot it left.
pub(super) fn saved_items(
    items: &[RuntimeItemStack],
    save: &mut BlockEntityItemSave<'_>,
) -> bbb_server_types::Result<NbtSummaryValue> {
    let mut list = Vec::new();
    for (slot, item) in items.iter().enumerate() {
        if item.is_empty() {
            continue;
        }
        let Ok(slot) = u8::try_from(slot) else {
            continue;
        };
        let Some(stack) = save.write(item)? else {
            continue;
        };
        let NbtSummaryValue::Compound(stack) = stack else {
            continue;
        };
        let mut entries = vec![entry(TAG_SLOT, NbtSummaryValue::Byte(slot as i8))];
        entries.extend(stack);
        list.push(NbtSummaryValue::Compound(entries));
    }
    Ok(NbtSummaryValue::List(list))
}

/// `ContainerHelper.loadAllItems` (`ContainerHelper.java:40-46`) with
/// `ItemStackWithSlot.isValidInContainer` (`ItemStackWithSlot.java:16-18`).
///
/// An element the codec rejects is vanilla's own silent drop: `listOrEmpty`
/// skips it rather than failing the load, so the slot stays empty instead of
/// the chest being lost. What this server *did* read and could not project is
/// not the same thing, and travels out through `load` as a named refusal.
pub(super) fn load_items(
    value: Option<&NbtSummaryValue>,
    items: &mut [RuntimeItemStack],
    load: &mut BlockEntityItemLoad<'_>,
) -> bbb_server_types::Result<()> {
    let Some(NbtSummaryValue::List(list)) = value else {
        return Ok(());
    };
    for element in list {
        // ContainerHelper.java:40-46: decode before checking the slot capacity.
        if let Some((slot, item)) = load.read_with_slot(element)? {
            if let Some(target) = items.get_mut(usize::from(slot)) {
                *target = item;
            }
        }
    }
    Ok(())
}

/// `Containers.dropContents(level, pos, container)`, which
/// `BlockEntity.preRemoveSideEffects` (`BlockEntity.java:233-237`) runs for a
/// `Container` — the whole of that method's 26.1 body.
///
/// One entry per slot in slot order, empty slots included: `dropItemStack`
/// draws a position for every slot it is handed (`Containers.java:25-30`), so
/// dropping the empty ones here would move the level's random source.
#[must_use]
pub fn drop_contents(
    chest: &ChestBlockEntity,
) -> Vec<bbb_server_types::item_stack::RuntimeItemStack> {
    chest
        .original_items()
        .iter()
        .map(bbb_server_types::item_stack::RuntimeItemStack::clone)
        .collect()
}

/// `CompoundContainer` (`CompoundContainer.java:11-56`): the first container's
/// slots followed by the second's, addressed as one.
///
/// Vanilla builds the object and lets every `getItem`/`setItem` split the
#[cfg(test)]
mod tests {
    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::VANILLA_BLOCK_CHEST_ID;
    use bbb_protocol::packets::BlockPos;
    use bbb_protocol::NbtSummaryValue;

    use crate::block_entity::{new_block_entity, unique_field};

    #[test]
    fn worldgen_loot_table_mutation_round_trips_without_rng() {
        let state = vanilla_block_default_state_id(VANILLA_BLOCK_CHEST_ID).unwrap();
        let pos = BlockPos { x: 1, y: 2, z: 3 };
        let mut chest = new_block_entity(pos, state).unwrap();
        chest.chest_mut().unwrap().set_loot_table(
            NbtSummaryValue::String("minecraft:simple_dungeon".into()),
            42,
        );
        let saved = chest
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        assert_eq!(
            unique_field(&saved, "LootTable"),
            Some(&NbtSummaryValue::String("minecraft:simple_dungeon".into()))
        );
        assert_eq!(
            unique_field(&saved, "LootTableSeed"),
            Some(&NbtSummaryValue::Long(42))
        );
        let loaded = crate::block_entity::load_test_fixture(pos, state, &saved)
            .expect("fixture codec succeeds")
            .unwrap();
        assert_eq!(
            loaded
                .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver(),)
                .expect("fixture codec succeeds"),
            saved
        );
        assert!(loaded.spawner().is_none());
    }
}
