//! `DecoratedPotBlockEntity` (`DecoratedPotBlockEntity.java:27-187`).
//!
//! One stack, analog from that stack, wobble block events, the `sherds`
//! decorations and the loot-table pair. Placement and projectile hits stay
//! outside this slice.

use bbb_protocol::item_types::{
    vanilla_item_type, vanilla_item_type_for_resource_id, VANILLA_ITEM_BRICK_ID,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_types::item_stack::RuntimeItemStack;

use super::components::{ComponentMapBuilder, ComponentMapReader};
use super::container_fields::ContainerLootTable;
use super::{entry, unique_field, BlockEntityItemLoad, BlockEntityItemSave};
use crate::inventory::redstone_signal_from_container;

/// `DecoratedPotBlockEntity.TAG_SHERDS` (`:28`).
const TAG_SHERDS: &str = "sherds";
/// `DecoratedPotBlockEntity.TAG_ITEM` (`:29`).
const TAG_ITEM: &str = "item";
/// `Container.getMaxStackSize` (`Container.java:34-36`).
const CONTAINER_MAX: i32 = 99;
/// `DecoratedPotBlockEntity.EVENT_POT_WOBBLES` (`:30`).
pub const EVENT_POT_WOBBLES: i32 = 1;

/// `DecoratedPotBlockEntity.WobbleStyle` (`:177-186`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WobbleStyle {
    Positive = 0,
    Negative = 1,
}

/// `DecoratedPotBlockEntity` (`:27-41`).
#[derive(Debug, Clone, PartialEq)]
pub struct DecoratedPotBlockEntity {
    item: RuntimeItemStack,
    /// `PotDecorations` back, left, right, front; `None` is the brick face.
    decorations: [Option<i32>; 4],
    loot_table: ContainerLootTable,
}

impl Default for DecoratedPotBlockEntity {
    fn default() -> Self {
        Self {
            item: RuntimeItemStack::empty(),
            decorations: [None; 4],
            loot_table: ContainerLootTable::default(),
        }
    }
}

impl DecoratedPotBlockEntity {
    // DecoratedPotBlockEntity.java:119-123 leaves the loot table untouched.
    pub(super) fn apply_implicit_components(
        &mut self,
        components: &mut ComponentMapReader<'_>,
    ) -> bbb_server_types::Result<()> {
        let decorations = components.read("minecraft:pot_decorations")?;
        self.decorations = decorations
            .as_ref()
            .and_then(read_decorations)
            .unwrap_or([None; 4]);
        self.item = components.copy_one()?;
        Ok(())
    }

    fn decorations_component(&self) -> NbtSummaryValue {
        NbtSummaryValue::List(
            self.ordered_decorations()
                .iter()
                .map(|item| {
                    let id = vanilla_item_type(*item)
                        .map_or("minecraft:brick", |entry| entry.resource_id);
                    NbtSummaryValue::string(id)
                })
                .collect(),
        )
    }

    /// `DecoratedPotBlockEntity.collectImplicitComponents` (`:112-116`).
    pub(super) fn collect_implicit_components(&self, components: &mut ComponentMapBuilder<'_>) {
        let decorations = Some(self.decorations_component());
        components.set("minecraft:pot_decorations", decorations);
        components.set_container(std::slice::from_ref(&self.item));
    }

    pub(super) const fn loot_table(&self) -> &ContainerLootTable {
        &self.loot_table
    }

    pub(super) fn randomizable_parts_mut(
        &mut self,
    ) -> (&mut ContainerLootTable, &mut [RuntimeItemStack]) {
        (&mut self.loot_table, std::slice::from_mut(&mut self.item))
    }

    /// `PotDecorations.ordered` (`PotDecorations.java:50-52`) as item type ids.
    #[must_use]
    pub fn ordered_decorations(&self) -> [i32; 4] {
        self.decorations
            .map(|item| item.unwrap_or(VANILLA_ITEM_BRICK_ID))
    }

    #[must_use]
    pub fn item(&self) -> &RuntimeItemStack {
        &self.item
    }

    #[must_use]
    pub fn items(&self) -> Vec<RuntimeItemStack> {
        vec![self.item.clone()]
    }

    pub fn original_items(&self) -> &[RuntimeItemStack] {
        std::slice::from_ref(&self.item)
    }

    #[cfg(test)]
    pub fn items_mut(&mut self) -> &mut [RuntimeItemStack] {
        std::slice::from_mut(&mut self.item)
    }

    /// `AbstractContainerMenu.getRedstoneSignalFromBlockEntity` (`:747-749`)
    /// over the one slot.
    pub fn redstone_signal(&self) -> bbb_server_types::Result<i32> {
        redstone_signal_from_container(&self.items())
    }

    pub fn set_item(&mut self, item: RuntimeItemStack) {
        if item.is_empty() {
            self.item = RuntimeItemStack::empty();
        } else {
            self.item = item;
        }
        self.loot_table = ContainerLootTable::default();
    }

    /// `DecoratedPotBlock.useItemOn` (`DecoratedPotBlock.java:110-123`): one
    /// item from the hand, stacking when the components match.
    pub fn try_insert_one(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        item: &RuntimeItemStack,
    ) -> bbb_server_types::Result<bool> {
        if !self.can_insert_one(resolver, item)? {
            return Ok(false);
        }
        if self.item.is_empty() {
            self.item = item.with_count(1);
        } else {
            self.item.set_count(self.item.count() + 1);
        }
        Ok(true)
    }

    pub fn can_insert_one(
        &self,
        resolver: &dyn PersistentItemStackHolderResolver,
        item: &RuntimeItemStack,
    ) -> bbb_server_types::Result<bool> {
        if self.loot_table.is_set() || item.is_empty() {
            return Ok(false);
        }
        if self.item.is_empty() {
            item.max_stack_size()?;
            return Ok(true);
        }
        if !self
            .item
            .same_item_same_effective_components(item, resolver)?
        {
            return Ok(false);
        }
        let max = self.item.max_stack_size()?.min(CONTAINER_MAX);
        Ok(self.item.count() < max)
    }

    pub fn overwrite_items(&mut self, items: &[RuntimeItemStack]) -> bool {
        if items.len() != 1 {
            return false;
        }
        self.set_item(items[0].clone());
        true
    }

    /// `DecoratedPotBlockEntity.saveAdditional` (`:44-53`).
    pub(super) fn save_additional(
        &self,
        save: &mut BlockEntityItemSave<'_>,
    ) -> bbb_server_types::Result<Vec<NbtSummaryEntry>> {
        let mut entries = Vec::new();
        if self.decorations != [None; 4] {
            let ordered = self
                .ordered_decorations()
                .iter()
                .filter_map(|item| vanilla_item_type(*item))
                .map(|entry| NbtSummaryValue::string(entry.resource_id))
                .collect();
            entries.push(entry(TAG_SHERDS, NbtSummaryValue::List(ordered)));
        }
        if !self.loot_table.try_save(&mut entries) && !self.item.is_empty() {
            // `output.store("item", ItemStack.CODEC, this.item)` (`:51`) writes
            // nothing when the encode fails: `ItemStack.CODEC` never answers a
            // partial value, so `TagValueOutput.store`'s partial arm
            // (`TagValueOutput.java:43`) leaves the field out.
            if let Some(item) = save.write(&self.item)? {
                entries.push(entry(TAG_ITEM, item));
            }
        }
        Ok(entries)
    }

    pub(super) fn load_additional(
        &mut self,
        record: &[NbtSummaryEntry],
        load: &mut BlockEntityItemLoad<'_>,
    ) -> bbb_server_types::Result<()> {
        self.decorations = unique_field(record, TAG_SHERDS)
            .and_then(read_decorations)
            .unwrap_or([None; 4]);
        self.loot_table = ContainerLootTable::try_load(record);
        self.item = if self.loot_table.is_set() {
            RuntimeItemStack::empty()
        } else {
            match unique_field(record, TAG_ITEM) {
                Some(value) => load_item(value, load)?.unwrap_or_else(RuntimeItemStack::empty),
                None => RuntimeItemStack::empty(),
            }
        };
        Ok(())
    }
}

/// `PotDecorations.CODEC` (`PotDecorations.java:25-28`): at most four
/// registered item ids, a brick or a missing tail reading as an empty face
/// (`:41-48`). A value it rejects leaves `PotDecorations.EMPTY`
/// (`DecoratedPotBlockEntity.java:58`).
fn read_decorations(value: &NbtSummaryValue) -> Option<[Option<i32>; 4]> {
    let NbtSummaryValue::List(items) = value else {
        return None;
    };
    if items.len() > 4 {
        return None;
    }
    let mut decorations = [None; 4];
    for (face, item) in items.iter().enumerate() {
        let NbtSummaryValue::String(id) = item else {
            return None;
        };
        let item = id
            .with_utf8(vanilla_item_type_for_resource_id)
            .flatten()?
            .id;
        decorations[face] = (item != VANILLA_ITEM_BRICK_ID).then_some(item);
    }
    Some(decorations)
}

fn load_item(
    value: &NbtSummaryValue,
    load: &mut BlockEntityItemLoad<'_>,
) -> bbb_server_types::Result<Option<RuntimeItemStack>> {
    load.read(value)
}

/// Contents `preRemoveSideEffects` drops. Sherds are a loot dynamic drop.
#[must_use]
pub fn drop_contents(
    pot: &DecoratedPotBlockEntity,
) -> Vec<bbb_server_types::item_stack::RuntimeItemStack> {
    pot.original_items()
        .iter()
        .map(bbb_server_types::item_stack::RuntimeItemStack::clone)
        .collect()
}
