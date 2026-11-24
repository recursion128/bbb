//! `ChiseledBookShelfBlockEntity` (`ChiseledBookShelfBlockEntity.java:24-138`).
//!
//! Six one-book slots, occupancy that lives on the block state, and analog
//! from `last_interacted_slot + 1`. Placement stays outside this slice.

use bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_tag_members_26_1;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue, PersistentRegistryTagMembers};
use bbb_server_types::item_stack::RuntimeItemStack;

use super::chest::{load_items, saved_items, TAG_ITEMS};
use super::components::{ComponentMapBuilder, ComponentMapReader};
use super::value_input::int_or;
use super::{entry, unique_field, BlockEntityItemLoad, BlockEntityItemSave};

/// `ChiseledBookShelfBlockEntity.MAX_BOOKS_IN_STORAGE` (`:25`).
pub const MAX_BOOKS_IN_STORAGE: usize = 6;
const TAG_LAST_INTERACTED_SLOT: &str = "last_interacted_slot";
const DEFAULT_LAST_INTERACTED_SLOT: i32 = -1;

/// `ChiseledBookShelfBlockEntity` (`:24-33`).
#[derive(Debug, Clone, PartialEq)]
pub struct ChiseledBookShelfBlockEntity {
    items: Vec<RuntimeItemStack>,
    last_interacted_slot: i32,
}

impl Default for ChiseledBookShelfBlockEntity {
    fn default() -> Self {
        Self {
            items: vec![RuntimeItemStack::empty(); MAX_BOOKS_IN_STORAGE],
            last_interacted_slot: DEFAULT_LAST_INTERACTED_SLOT,
        }
    }
}

impl ChiseledBookShelfBlockEntity {
    /// `ChiseledBookShelfBlockEntity.collectImplicitComponents` (`ChiseledBookShelfBlockEntity.java:129-132`).
    pub(super) fn collect_implicit_components(&self, components: &mut ComponentMapBuilder<'_>) {
        components.set_container(&self.items);
    }

    /// `ChiseledBookShelfBlockEntity.applyImplicitComponents` (`ChiseledBookShelfBlockEntity.java:123-126`).
    pub(super) fn apply_implicit_components(
        &mut self,
        components: &mut ComponentMapReader<'_>,
    ) -> bbb_server_types::Result<()> {
        components.copy_container_into(&mut self.items)
    }

    pub(super) fn dropped_slots_mut(&mut self) -> &mut [RuntimeItemStack] {
        &mut self.items
    }

    #[must_use]
    pub fn items(&self) -> Vec<RuntimeItemStack> {
        self.items.to_vec()
    }

    pub fn original_items(&self) -> &[RuntimeItemStack] {
        &self.items
    }

    #[cfg(test)]
    pub fn items_mut(&mut self) -> &mut [RuntimeItemStack] {
        &mut self.items
    }

    #[must_use]
    pub const fn last_interacted_slot(&self) -> i32 {
        self.last_interacted_slot
    }

    /// `ChiseledBookShelfBlock.getAnalogOutputSignal` (`ChiseledBookShelfBlock.java:191-196`).
    #[must_use]
    pub const fn redstone_signal(&self) -> i32 {
        self.last_interacted_slot + 1
    }

    /// `ChiseledBookShelfBlockEntity.acceptsItemType` (`:74-76`).
    #[must_use]
    pub fn accepts_item(item: &RuntimeItemStack) -> bool {
        item_is_bookshelf_book(item)
    }

    /// `getMaxStackSize` is 1 (`:69-71`), so a hopper/dropper only inserts
    /// into a vacant slot.
    #[must_use]
    pub fn has_empty_slot(&self) -> bool {
        self.items.iter().any(|item| item.is_empty())
    }

    /// `setItem` (`:90-97`) for a book, or `removeItem` (`:79-87`) for empty.
    pub fn set_slot(&mut self, slot: usize, item: RuntimeItemStack) -> bool {
        let accepts = Self::accepts_item(&item);
        self.set_original_slot(slot, item, accepts)
    }

    pub fn set_original_slot(
        &mut self,
        slot: usize,
        item: RuntimeItemStack,
        accepts: bool,
    ) -> bool {
        if slot >= self.items.len() {
            return false;
        }
        let changed = if accepts {
            self.items[slot] = item;
            true
        } else if item.is_empty() {
            let occupied = !self.items[slot].is_empty();
            self.items[slot] = RuntimeItemStack::empty();
            occupied
        } else {
            false
        };
        if changed {
            self.last_interacted_slot = i32::try_from(slot).expect("slot is 0-5");
        }
        changed
    }

    /// Replace the six slots after a hopper/dropper walk; answers the last
    /// slot that actually moved, which `updateState` needs.
    pub fn overwrite_items(
        &mut self,
        items: &[RuntimeItemStack],
        accepts: impl Fn(&RuntimeItemStack) -> bool,
    ) -> Option<usize> {
        if items.len() != self.items.len() {
            return None;
        }
        let mut changed = None;
        for (slot, next) in items.iter().enumerate() {
            let previous = &self.items[slot];
            let prev_empty = previous.is_empty();
            let next_empty = next.is_empty();
            if prev_empty && next_empty {
                continue;
            }
            if !next_empty && !accepts(next) {
                continue;
            }
            if previous.matches(next) {
                continue;
            }
            if next_empty {
                self.items[slot] = RuntimeItemStack::empty();
            } else {
                self.items[slot] = next.clone();
            }
            changed = Some(slot);
        }
        if let Some(slot) = changed {
            self.last_interacted_slot = i32::try_from(slot).expect("slot is 0-5");
        }
        changed
    }

    pub(super) fn save_additional(
        &self,
        save: &mut BlockEntityItemSave<'_>,
    ) -> bbb_server_types::Result<Vec<NbtSummaryEntry>> {
        Ok(vec![
            entry(TAG_ITEMS, saved_items(&self.items, save)?),
            entry(
                TAG_LAST_INTERACTED_SLOT,
                NbtSummaryValue::Int(self.last_interacted_slot),
            ),
        ])
    }

    pub(super) fn load_additional(
        &mut self,
        record: &[NbtSummaryEntry],
        load: &mut BlockEntityItemLoad<'_>,
    ) -> bbb_server_types::Result<()> {
        self.items = vec![RuntimeItemStack::empty(); MAX_BOOKS_IN_STORAGE];
        load_items(unique_field(record, TAG_ITEMS), &mut self.items, load)?;
        self.last_interacted_slot = int_or(
            record,
            TAG_LAST_INTERACTED_SLOT,
            DEFAULT_LAST_INTERACTED_SLOT,
        );
        Ok(())
    }
}

/// `ItemTags.BOOKSHELF_BOOKS` (`ChiseledBookShelfBlock.java:86`).
pub const BOOKSHELF_BOOKS_TAG: &str = "minecraft:bookshelf_books";

/// `ItemTags.BOOKSHELF_BOOKS` checked against a caller-supplied live tag table.
#[must_use]
pub fn item_is_bookshelf_book_with(
    item: &RuntimeItemStack,
    tag_contains: impl FnOnce(&str, &str, &str) -> bool,
) -> bool {
    if item.count() <= 0 {
        return false;
    }
    item.item_resource_id()
        .is_some_and(|id| tag_contains("minecraft:item", BOOKSHELF_BOOKS_TAG, id))
}

/// `ItemTags.BOOKSHELF_BOOKS` (`ItemTags.java` / 26.1 built-in tag snapshot).
#[must_use]
pub fn item_is_bookshelf_book(item: &RuntimeItemStack) -> bool {
    item_is_bookshelf_book_with(item, |registry, tag, id| {
        matches!(
            vanilla_persistent_registry_tag_members_26_1(registry, tag),
            PersistentRegistryTagMembers::Present(members)
                if members.iter().any(|member| member == id)
        )
    })
}

/// Contents `preRemoveSideEffects` drops.
#[must_use]
pub fn drop_contents(
    bookshelf: &ChiseledBookShelfBlockEntity,
) -> Vec<bbb_server_types::item_stack::RuntimeItemStack> {
    bookshelf
        .original_items()
        .iter()
        .map(bbb_server_types::item_stack::RuntimeItemStack::clone)
        .collect()
}
