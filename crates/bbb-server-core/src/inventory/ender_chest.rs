use super::ENDER_CHEST_SIZE;
use crate::block_entity::BlockEntity;
use crate::world::maps::ServerMapStore;
use bbb_server_types::item_stack::RuntimeItemStack;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct EnderChestContainer {
    slots: Vec<RuntimeItemStack>,
    active_chest: Option<(bbb_server_types::save::Java26LevelKey, BlockEntity)>,
}

impl Default for EnderChestContainer {
    fn default() -> Self {
        Self {
            slots: (0..ENDER_CHEST_SIZE)
                .map(|_| RuntimeItemStack::empty())
                .collect(),
            active_chest: None,
        }
    }
}

impl EnderChestContainer {
    pub(super) fn slots(&self) -> Vec<RuntimeItemStack> {
        self.slots.clone()
    }

    pub(super) fn slot_slice(&self) -> &[RuntimeItemStack] {
        &self.slots
    }

    pub(super) fn slot_ref(&self, index: usize) -> Option<&RuntimeItemStack> {
        self.slots.get(index)
    }

    pub(super) fn get(&self, index: usize) -> Option<RuntimeItemStack> {
        self.slots.get(index).cloned()
    }

    pub(super) fn set_unchecked(&mut self, index: usize, item: RuntimeItemStack) {
        self.slots[index] = item;
    }

    pub(super) fn set(
        &mut self,
        index: usize,
        mut item: RuntimeItemStack,
    ) -> bbb_server_types::Result<bool> {
        if index >= ENDER_CHEST_SIZE {
            return Ok(false);
        }
        limit_size(&mut item)?;
        self.slots[index] = item;
        Ok(true)
    }

    pub(super) fn install(&mut self, items: Vec<RuntimeItemStack>) -> bool {
        if items.len() != ENDER_CHEST_SIZE {
            return false;
        }
        self.slots = items;
        true
    }

    pub(super) fn active_chest(
        &self,
    ) -> Option<(bbb_server_types::save::Java26LevelKey, BlockEntity)> {
        self.active_chest.clone()
    }

    pub(super) fn set_active_chest(
        &mut self,
        chest: Option<(bbb_server_types::save::Java26LevelKey, BlockEntity)>,
    ) {
        self.active_chest = chest;
    }
}

impl super::PlayerInventoryMenu {
    pub fn is_ender_chest_menu(&self) -> bool {
        self.ender_chest_menu
    }

    pub(crate) fn prepare_ender_chest_menu(
        &mut self,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        maps: Option<&mut ServerMapStore>,
        container_id: i32,
        pos: bbb_protocol::packets::BlockPos,
    ) -> bbb_server_types::Result<Option<super::PreparedMenu>> {
        let opened = self.prepare_reserved_menu(
            resolver,
            maps,
            container_id,
            super::MenuLayout::Chest {
                rows: super::CHEST_MENU_ROWS,
            },
            super::ContainerAnchor {
                positions: vec![pos],
            },
            self.ender_chest_slots(),
        )?;
        if opened.is_some() {
            self.ender_chest_menu = true;
        }
        Ok(opened)
    }
}

pub(super) fn limit_size(item: &mut RuntimeItemStack) -> bbb_server_types::Result<()> {
    let limit = super::container_max_stack_size(item)?;
    if item.count() > limit {
        item.set_count(limit);
    }
    Ok(())
}
