use super::{BlockEntity, BlockEntityPayload};
use bbb_server_types::item_stack::RuntimeItemStack;

impl BlockEntity {
    /// Raw slots; the level owns getItem/isEmpty's loot unpack and callbacks.
    pub fn container_items(&self) -> Option<Vec<RuntimeItemStack>> {
        self.container_slots().map(<[_]>::to_vec)
    }

    pub(crate) fn loot_slots(&self) -> Option<&[RuntimeItemStack]> {
        match &self.payload {
            BlockEntityPayload::Jukebox(jukebox) => Some(jukebox.original_items()),
            _ => self.container_slots(),
        }
    }

    pub(crate) fn container_slots(&self) -> Option<&[RuntimeItemStack]> {
        if matches!(
            self.payload,
            BlockEntityPayload::Campfire(_)
                | BlockEntityPayload::Lectern(_)
                | BlockEntityPayload::Jukebox(_)
        ) {
            return None;
        }
        let items = self.payload.original_items();
        (!items.is_empty()).then_some(items)
    }

    pub fn install_container_originals(&mut self, items: Vec<RuntimeItemStack>) -> bool {
        if matches!(
            self.payload,
            BlockEntityPayload::Campfire(_)
                | BlockEntityPayload::Lectern(_)
                | BlockEntityPayload::Jukebox(_)
        ) {
            return false;
        }
        let slots = self.payload.original_items_mut();
        if slots.is_empty() || slots.len() != items.len() {
            return false;
        }
        for (slot, item) in slots.iter_mut().zip(items) {
            *slot = item;
        }
        true
    }

    pub fn can_open_container(
        &self,
        is_spectator: bool,
        key: &RuntimeItemStack,
        registries: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
    ) -> bbb_server_types::Result<bool> {
        match &self.payload {
            BlockEntityPayload::Chest(c) => c.can_open(is_spectator, key, registries),
            BlockEntityPayload::Barrel(c) => c.can_open(is_spectator, key, registries),
            BlockEntityPayload::Dispenser(c) => c.can_open(is_spectator, key, registries),
            BlockEntityPayload::Hopper(c) => c.can_open(is_spectator, key, registries),
            BlockEntityPayload::Crafter(c) => c.can_open(is_spectator, key, registries),
            BlockEntityPayload::ShulkerBox(c) => c.can_open_record(is_spectator, key, registries),
            BlockEntityPayload::Furnace(c) => c.can_open(is_spectator, key, registries),
            BlockEntityPayload::BrewingStand(c) => c.can_open(is_spectator, key, registries),
            BlockEntityPayload::Beacon(c) => Ok(c.can_open(is_spectator, key, registries)),
            _ => Ok(false),
        }
    }

    pub fn container_custom_name(&self) -> Option<&bbb_protocol::NbtSummaryValue> {
        match &self.payload {
            BlockEntityPayload::Chest(c) => c.custom_name(),
            BlockEntityPayload::Barrel(c) => c.custom_name(),
            BlockEntityPayload::Dispenser(c) => c.custom_name(),
            BlockEntityPayload::Hopper(c) => c.custom_name(),
            BlockEntityPayload::Crafter(c) => c.custom_name(),
            BlockEntityPayload::ShulkerBox(c) => c.custom_name(),
            BlockEntityPayload::Furnace(c) => c.custom_name(),
            BlockEntityPayload::BrewingStand(c) => c.custom_name(),
            BlockEntityPayload::Beacon(c) => c.custom_name(),
            _ => None,
        }
    }

    pub fn nameable_custom_name(&self) -> Option<Option<&bbb_protocol::NbtSummaryValue>> {
        Some(match &self.payload {
            BlockEntityPayload::Chest(c) => c.custom_name(),
            BlockEntityPayload::Barrel(c) => c.custom_name(),
            BlockEntityPayload::Dispenser(c) => c.custom_name(),
            BlockEntityPayload::Hopper(c) => c.custom_name(),
            BlockEntityPayload::Crafter(c) => c.custom_name(),
            BlockEntityPayload::ShulkerBox(c) => c.custom_name(),
            BlockEntityPayload::Furnace(c) => c.custom_name(),
            BlockEntityPayload::BrewingStand(c) => c.custom_name(),
            BlockEntityPayload::Banner(c) => c.custom_name(),
            BlockEntityPayload::Beacon(c) => c.custom_name(),
            BlockEntityPayload::EnchantingTable(c) => c.custom_name(),
            _ => return None,
        })
    }

    /// ItemStack count mutation, without Container.setItem's side effects.
    pub fn set_container_item_count(
        &mut self,
        slot: usize,
        count: i32,
    ) -> bbb_server_types::Result<()> {
        let items = match &mut self.payload {
            BlockEntityPayload::Chest(c) => c.randomizable_parts_mut().1,
            BlockEntityPayload::Barrel(c) => c.randomizable_parts_mut().1,
            BlockEntityPayload::Dispenser(c) => c.randomizable_parts_mut().1,
            BlockEntityPayload::Hopper(c) => c.randomizable_parts_mut().1,
            BlockEntityPayload::Crafter(c) => c.randomizable_parts_mut().1,
            BlockEntityPayload::ShulkerBox(c) => c.randomizable_parts_mut().1,
            BlockEntityPayload::DecoratedPot(c) => c.randomizable_parts_mut().1,
            BlockEntityPayload::Furnace(c) => c.dropped_slots_mut(),
            BlockEntityPayload::BrewingStand(c) => c.dropped_slots_mut(),
            BlockEntityPayload::ChiseledBookshelf(c) => c.dropped_slots_mut(),
            BlockEntityPayload::Shelf(c) => c.dropped_slots_mut(),
            _ => return Ok(()),
        };
        if let Some(item) = items.get_mut(slot) {
            item.set_count(count);
        }
        Ok(())
    }
}
