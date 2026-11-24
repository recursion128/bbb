//! What block loot reads from a block entity: `collectComponents`, the dynamic
//! drops `Block.getDrops` overrides register from it, the `RandomizableContainer`
//! half, and the object `playerDestroy` still holds once the block is gone.

use bbb_protocol::packets::ItemStackSummary;
use bbb_protocol::NbtSummaryValue;
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_types::item_stack::RuntimeItemStack;

use super::components::ComponentMapBuilder;
use super::{
    entry, BlockEntity, BlockEntityItemProjection, BlockEntityPayload, BlockEntityRemovalDrops,
    CollectedComponents, ContainerLootTable,
};

impl BlockEntity {
    pub fn apply_components_from_item_stack(
        &mut self,
        source: &RuntimeItemStack,
        projection: BlockEntityItemProjection<'_>,
    ) -> bbb_server_types::Result<()> {
        let mut components = super::components::ComponentMapReader::new(source, projection);
        self.apply_implicit_components(&mut components)?;
        self.components = components.retained();
        Ok(())
    }

    /// `BlockEntity.collectComponents` (`BlockEntity.java:309-314`), with the
    /// stacks a container component holds written through `projection`.
    #[must_use]
    pub fn collect_components(
        &self,
        projection: BlockEntityItemProjection<'_>,
    ) -> CollectedComponents {
        let mut components = ComponentMapBuilder::new(&self.components, projection);
        self.collect_implicit_components(&mut components);
        components.build()
    }

    /// `new ItemStack(state.getBlock())` after
    /// `applyComponents(blockEntity.collectComponents())` (`ShulkerBoxBlock.java:112-113`)
    /// as the persistent stack. `None` for a block entity that is not a shulker box.
    #[must_use]
    pub fn shulker_box_drop_stack(
        &self,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> Option<NbtSummaryValue> {
        self.shulker_box()?;
        let item = bbb_protocol::block_types::vanilla_block_type(
            bbb_protocol::block_state_types::vanilla_block_state_block_type_id(self.block_state)?,
        )?
        .resource_id;
        let collected = self.collect_components(BlockEntityItemProjection::with_resolver(resolver));
        let components = collected
            .entries()
            .iter()
            .filter_map(|component| {
                let value = component.value.as_ref().ok()?;
                Some(entry(&component.id, value.clone()))
            })
            .collect();
        let stack = NbtSummaryValue::Compound(vec![
            entry("id", NbtSummaryValue::string(item)),
            entry("count", NbtSummaryValue::Int(1)),
            entry("components", NbtSummaryValue::Compound(components)),
        ]);
        Some(stack)
    }

    /// The `LootParams.Builder.withDynamicDrop` registration `Block.getDrops`
    /// makes for this object, answered for one `minecraft:dynamic` name:
    /// `ShulkerBoxBlock.CONTENTS` (`ShulkerBoxBlock.java:126-137`) and
    /// `DecoratedPotBlock.SHERDS_DYNAMIC_DROP_ID` (`DecoratedPotBlock.java:181-192`).
    /// `None` is a name nothing registered, which `LootParams.addDynamicDrops`
    /// (`LootParams.java:35-40`) answers with no stack.
    #[must_use]
    pub fn dynamic_drop(&self, name: &str) -> Option<Result<Vec<ItemStackSummary>, String>> {
        match (&self.payload, name) {
            (BlockEntityPayload::ShulkerBox(shulker), "minecraft:contents") => {
                // `getItem` unpacks a pending loot table into the box mid-roll
                // (`RandomizableContainerBlockEntity.java:54-57`).
                Some(if shulker.has_loot_table() {
                    Err("a shulker box that still names a loot table would fill mid-roll".into())
                } else {
                    Ok(shulker
                        .original_items()
                        .iter()
                        .map(|item| item.summary().clone())
                        .collect())
                })
            }
            (BlockEntityPayload::DecoratedPot(pot), "minecraft:sherds") => Some(Ok(pot
                .ordered_decorations()
                .iter()
                .map(|item| ItemStackSummary {
                    item_id: Some(*item),
                    count: 1,
                    ..ItemStackSummary::empty()
                })
                .collect())),
            _ => None,
        }
    }

    /// The `RandomizableContainer` half of this object: its loot-table pair and
    /// the slots `LootTable.fill` writes (`LootTable.java:146-165`). `None` for
    /// a type that does not implement the interface.
    pub fn randomizable_container_mut(
        &mut self,
    ) -> Option<(&mut ContainerLootTable, &mut [RuntimeItemStack])> {
        match &mut self.payload {
            BlockEntityPayload::Chest(chest) => Some(chest.randomizable_parts_mut()),
            BlockEntityPayload::Barrel(barrel) => Some(barrel.randomizable_parts_mut()),
            BlockEntityPayload::Dispenser(dispenser) => Some(dispenser.randomizable_parts_mut()),
            BlockEntityPayload::Hopper(hopper) => Some(hopper.randomizable_parts_mut()),
            BlockEntityPayload::Crafter(crafter) => Some(crafter.randomizable_parts_mut()),
            BlockEntityPayload::ShulkerBox(shulker) => Some(shulker.randomizable_parts_mut()),
            BlockEntityPayload::DecoratedPot(pot) => Some(pot.randomizable_parts_mut()),
            _ => None,
        }
    }

    #[must_use]
    pub fn container_loot_table_id(&self) -> Option<&str> {
        let table = match &self.payload {
            BlockEntityPayload::Chest(container) => container.loot_table(),
            BlockEntityPayload::Barrel(container) => container.loot_table(),
            BlockEntityPayload::Dispenser(container) => container.loot_table(),
            BlockEntityPayload::Hopper(container) => container.loot_table(),
            BlockEntityPayload::Crafter(container) => container.loot_table(),
            BlockEntityPayload::ShulkerBox(container) => container.loot_table(),
            BlockEntityPayload::DecoratedPot(container) => container.loot_table(),
            _ => return None,
        };
        table.table_id()
    }

    /// `RandomizableContainer.setLootTable(lootTable, seed)`
    /// (`RandomizableContainer.java:29-32`).
    pub fn set_container_loot_table(&mut self, id: &str, seed: i64) -> bool {
        let Some((loot, _)) = self.randomizable_container_mut() else {
            return false;
        };
        loot.set(&NbtSummaryValue::string(id), seed);
        true
    }

    /// The `setSlotState(slot, true)` `CrafterBlockEntity.setItem` runs ahead of
    /// the write when `LootTable.fill` lands on a disabled slot
    /// (`CrafterBlockEntity.java:172-178`), answering whether it ran and so
    /// called `setChanged`.
    pub fn enable_filled_crafter_slot(&mut self, slot: usize) -> bool {
        let BlockEntityPayload::Crafter(crafter) = &mut self.payload else {
            return false;
        };
        crafter.enable_disabled_slot(slot)
    }

    pub fn set_filled_original(&mut self, slot: usize, stack: RuntimeItemStack) -> bool {
        let marks = !matches!(
            self.payload,
            BlockEntityPayload::Hopper(_) | BlockEntityPayload::DecoratedPot(_)
        );
        if let Some(target) = self
            .randomizable_container_mut()
            .and_then(|(_, slots)| slots.get_mut(slot))
        {
            *target = stack;
        }
        marks
    }

    /// Whether removing this object with side effects unpacks a loot table:
    /// `Containers.dropContents` reads every slot through
    /// `RandomizableContainerBlockEntity.getItem` (`Containers.java:22-26`,
    /// `RandomizableContainerBlockEntity.java:54-57`).
    #[must_use]
    pub fn removal_unpacks_loot(&self, old_state: i32) -> bool {
        let dropped = matches!(
            self.pre_remove_side_effects(old_state),
            BlockEntityRemovalDrops::ContainerSlots(_)
        );
        let loot = match &self.payload {
            BlockEntityPayload::Chest(chest) => chest.loot_table(),
            BlockEntityPayload::Barrel(barrel) => barrel.loot_table(),
            BlockEntityPayload::Dispenser(dispenser) => dispenser.loot_table(),
            BlockEntityPayload::Hopper(hopper) => hopper.loot_table(),
            BlockEntityPayload::Crafter(crafter) => crafter.loot_table(),
            BlockEntityPayload::DecoratedPot(pot) => pot.loot_table(),
            _ => return false,
        };
        dropped && loot.is_set()
    }
}
