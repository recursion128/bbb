//! `ShelfBlockEntity` (`ShelfBlockEntity.java:31-138`).
//!
//! Three slots, analog from those slots, and a swap with the player's hand.
//! Multi-shelf `SIDE_CHAIN_PART` linking stays outside this slice: a powered
//! shelf still swaps its three slots with hotbar 6–8 as an isolated chain of
//! one. Placement also stays out.

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::item_stack::RuntimeItemStack;

use super::chest::{load_items, saved_items, TAG_ITEMS};
use super::components::{ComponentMapBuilder, ComponentMapReader};
use super::value_input::boolean_or;
use super::{entry, unique_field, BlockEntityItemLoad, BlockEntityItemSave};
use crate::inventory::redstone_signal_from_container;

/// `ShelfBlockEntity.MAX_ITEMS` (`:32`).
pub const MAX_ITEMS: usize = 3;
const TAG_ALIGN_ITEMS_TO_BOTTOM: &str = "align_items_to_bottom";

/// `ShelfBlockEntity` (`:31-40`).
#[derive(Debug, Clone, PartialEq)]
pub struct ShelfBlockEntity {
    items: Vec<RuntimeItemStack>,
    align_items_to_bottom: bool,
}

impl Default for ShelfBlockEntity {
    fn default() -> Self {
        Self {
            items: vec![RuntimeItemStack::empty(); MAX_ITEMS],
            align_items_to_bottom: false,
        }
    }
}

impl ShelfBlockEntity {
    /// `ShelfBlockEntity.collectImplicitComponents` (`ShelfBlockEntity.java:110-113`).
    pub(super) fn collect_implicit_components(&self, components: &mut ComponentMapBuilder<'_>) {
        components.set_container(&self.items);
    }

    /// `ShelfBlockEntity.applyImplicitComponents` (`ShelfBlockEntity.java:104-107`).
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

    /// `AbstractContainerMenu.getRedstoneSignalFromBlockEntity` over three slots.
    pub fn redstone_signal(&self) -> bbb_server_types::Result<i32> {
        redstone_signal_from_container(&self.items())
    }

    /// `ShelfBlockEntity.swapItemNoUpdate` (`:81-85`).
    pub fn swap_item(&mut self, slot: usize, held: RuntimeItemStack) -> Option<RuntimeItemStack> {
        if slot >= self.items.len() {
            return None;
        }
        let previous = std::mem::replace(&mut self.items[slot], normalize(held)).clone();
        Some(previous)
    }

    pub fn overwrite_items(&mut self, items: &[RuntimeItemStack]) -> bool {
        if items.len() != self.items.len() {
            return false;
        }
        self.items = items.to_vec();
        true
    }

    pub(super) fn save_additional(
        &self,
        save: &mut BlockEntityItemSave<'_>,
    ) -> bbb_server_types::Result<Vec<NbtSummaryEntry>> {
        Ok(vec![
            entry(TAG_ITEMS, saved_items(&self.items, save)?),
            entry(
                TAG_ALIGN_ITEMS_TO_BOTTOM,
                NbtSummaryValue::Byte(i8::from(self.align_items_to_bottom)),
            ),
        ])
    }

    pub(super) fn load_additional(
        &mut self,
        record: &[NbtSummaryEntry],
        load: &mut BlockEntityItemLoad<'_>,
    ) -> bbb_server_types::Result<()> {
        self.items = vec![RuntimeItemStack::empty(); MAX_ITEMS];
        load_items(unique_field(record, TAG_ITEMS), &mut self.items, load)?;
        self.align_items_to_bottom = boolean_or(record, TAG_ALIGN_ITEMS_TO_BOTTOM, false);
        Ok(())
    }
}

fn normalize(item: RuntimeItemStack) -> RuntimeItemStack {
    if item.is_empty() {
        RuntimeItemStack::empty()
    } else {
        item
    }
}

/// Contents `preRemoveSideEffects` drops.
#[must_use]
pub fn drop_contents(
    shelf: &ShelfBlockEntity,
) -> Vec<bbb_server_types::item_stack::RuntimeItemStack> {
    shelf
        .original_items()
        .iter()
        .map(bbb_server_types::item_stack::RuntimeItemStack::clone)
        .collect()
}
