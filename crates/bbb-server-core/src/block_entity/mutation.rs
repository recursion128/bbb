use super::*;
use crate::chunk::block_entities::BlockEntities;
use bbb_server_types::item_stack::RuntimeItemStack;

impl BlockEntityPayload {
    pub(super) fn original_items(&self) -> &[RuntimeItemStack] {
        match self {
            Self::Chest(value) => value.original_items(),
            Self::Barrel(value) => value.original_items(),
            Self::Dispenser(value) => value.original_items(),
            Self::Hopper(value) => value.original_items(),
            Self::Crafter(value) => value.original_items(),
            Self::ShulkerBox(value) => value.original_items(),
            Self::DecoratedPot(value) => value.original_items(),
            Self::Furnace(value) => value.original_items(),
            Self::BrewingStand(value) => value.original_items(),
            Self::ChiseledBookshelf(value) => value.original_items(),
            Self::Shelf(value) => value.original_items(),
            Self::Campfire(value) => value.original_items(),
            Self::Lectern(value) => value.original_items(),
            Self::Jukebox(value) => value.original_items(),
            _ => &[],
        }
    }

    pub(super) fn original_items_mut(&mut self) -> &mut [RuntimeItemStack] {
        match self {
            Self::Chest(value) => value.randomizable_parts_mut().1,
            Self::Barrel(value) => value.randomizable_parts_mut().1,
            Self::Dispenser(value) => value.randomizable_parts_mut().1,
            Self::Hopper(value) => value.randomizable_parts_mut().1,
            Self::Crafter(value) => value.randomizable_parts_mut().1,
            Self::ShulkerBox(value) => value.randomizable_parts_mut().1,
            Self::DecoratedPot(value) => value.randomizable_parts_mut().1,
            Self::Furnace(value) => value.dropped_slots_mut(),
            Self::BrewingStand(value) => value.dropped_slots_mut(),
            Self::ChiseledBookshelf(value) => value.dropped_slots_mut(),
            Self::Shelf(value) => value.dropped_slots_mut(),
            Self::Campfire(value) => value.dropped_slots_mut(),
            Self::Lectern(value) => value.original_items_mut(),
            Self::Jukebox(value) => value.original_items_mut(),
            _ => &mut [],
        }
    }
}

impl BlockEntity {
    /// The live block entity standing for this one, else this value.
    pub fn resolve_instance(&self, entities: BlockEntities<'_>) -> Self {
        entities
            .get(self.block_pos())
            .filter(|current| current.same_position_and_type(self))
            .cloned()
            .unwrap_or_else(|| self.clone())
    }
}
