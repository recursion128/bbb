//! Opening, closing and the buttons of the menus layered over the inventory menu.

use crate::world::maps::ServerMapStore;
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_types::item_stack::RuntimeItemStack;

#[cfg(test)]
use super::{OpenedMenu, MENU_CONTAINER_ID};
use bbb_server_types::Result;

use super::{
    ClosedMenu, ContainerAnchor, MenuLayout, MenuState, MenuSync, ParkedOpenMenu,
    PlayerInventoryMenu, PreparedMenu, CONTAINER_COUNTER_WRAP,
};

impl PlayerInventoryMenu {
    pub(crate) fn vehicle_container(&self) -> Option<(crate::world::LevelId, i32)> {
        self.menu.vehicle_container
    }

    pub(crate) fn prepare_vehicle_container(
        &mut self,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        container_id: i32,
        layout: MenuLayout,
        dimension: crate::world::LevelId,
        entity_id: i32,
        items: Vec<RuntimeItemStack>,
    ) -> Result<Option<PreparedMenu>> {
        let prepared = self.prepare_container_menu(
            resolver,
            None,
            container_id,
            layout,
            ContainerAnchor {
                positions: Vec::new(),
            },
            items,
        )?;
        if prepared.is_some() {
            self.container_anchor = None;
            self.menu.vehicle_container = Some((dimension, entity_id));
        }
        Ok(prepared)
    }

    pub fn bind_container_instances(&mut self, instances: Vec<crate::block_entity::BlockEntity>) {
        self.container_instances = instances;
    }

    pub fn container_instances(&self) -> &[crate::block_entity::BlockEntity] {
        &self.container_instances
    }

    /// `AbstractContainerMenu.containerId` (:59) of the menu clicks address,
    /// which `handleContainerClick` (:1918) compares the packet against.
    pub fn container_id(&self) -> i32 {
        self.menu.container_id
    }

    /// Runs `body` with `Player.inventoryMenu` (`Player.java:152`) as the
    /// current menu, whatever `containerMenu` points at.
    ///
    /// Vanilla names the two menus apart at every call site, so a handler that
    /// says `this.player.inventoryMenu` reaches the player's own menu even
    /// while a chest is open. Here one of the two is always the current one,
    /// so naming the other means swapping them for the length of the call.
    pub fn with_inventory_menu<R>(&mut self, body: impl FnOnce(&mut Self) -> R) -> R {
        let parked = self.begin_inventory_menu();
        let result = body(self);
        self.end_inventory_menu(parked);
        result
    }

    pub fn begin_inventory_menu(&mut self) -> Option<ParkedOpenMenu> {
        let inventory_menu = self.suspended_inventory_menu.take()?;
        self.menu_epoch = self.menu_epoch.wrapping_add(1);
        Some(ParkedOpenMenu {
            menu: std::mem::replace(&mut self.menu, inventory_menu),
            container: std::mem::take(&mut self.container),
            epoch: self.menu_epoch,
        })
    }

    pub fn end_inventory_menu(&mut self, parked: Option<ParkedOpenMenu>) {
        let Some(parked) = parked else {
            return;
        };
        if self.menu_epoch != parked.epoch {
            return;
        }
        self.suspended_inventory_menu = Some(std::mem::replace(&mut self.menu, parked.menu));
        self.container = parked.container;
        self.menu_epoch = self.menu_epoch.wrapping_add(1);
    }

    /// The three writes `handleSetCreativeModeSlot` makes (:2013-2015), all of
    /// which name `ServerPlayer.inventoryMenu`: the slot, the menu's own
    /// mirror of it, and the broadcast that therefore finds nothing to report.
    pub fn creative_set_inventory_menu_slot(
        &mut self,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        slot: i16,
        item: &RuntimeItemStack,
    ) -> Result<Vec<MenuSync>> {
        self.with_inventory_menu(|menu| {
            menu.set_by_player(resolver, None, slot, item.clone())?;
            menu.set_remote_slot(slot, item);
            menu.broadcast_changes(resolver, None)
        })
    }

    /// Which block entities the open menu's `Container` is, or `None` when the
    /// open menu has no container of its own.
    pub fn open_container_anchor(&self) -> Option<&ContainerAnchor> {
        self.container_anchor.as_ref()
    }

    /// `AbstractContainerMenu.clickMenuButton` (`AbstractContainerMenu.java:301-303`)
    /// as `LecternMenu` (`LecternMenu.java:39-72`) overrides it.
    ///
    /// Answers whether the click was accepted. Page jumps are applied to the
    /// returned page; taking the book leaves it in `container[0]` emptied and
    /// the taken stack in `taken`.
    pub fn click_menu_button(
        &mut self,
        button_id: i32,
        may_build: bool,
    ) -> (bool, Option<bbb_server_types::item_stack::RuntimeItemStack>) {
        if self.menu.layout != MenuLayout::Lectern {
            return (false, None);
        }
        if button_id >= 100 {
            return (true, None);
        }
        match button_id {
            1 | 2 => (true, None),
            3 => {
                if !may_build {
                    return (false, None);
                }
                if self.container.is_empty() {
                    return (true, None);
                }
                let taken = std::mem::replace(&mut self.container[0], RuntimeItemStack::empty());
                (true, Some(taken))
            }
            _ => (false, None),
        }
    }

    /// The page a lectern-menu button asked for, or `None` when the button
    /// is not a page turn. The block entity clamps and pulses.
    #[must_use]
    pub fn lectern_button_page(&self, button_id: i32, current_page: i32) -> Option<i32> {
        if self.menu.layout != MenuLayout::Lectern {
            return None;
        }
        if button_id >= 100 {
            return Some(button_id - 100);
        }
        match button_id {
            1 => Some(current_page - 1),
            2 => Some(current_page + 1),
            _ => None,
        }
    }

    /// Values of the open container's current slots.
    pub fn container_items(&self) -> Vec<RuntimeItemStack> {
        if self.ender_chest_menu {
            return self.ender_chest.slots();
        }
        self.container.clone()
    }

    /// Borrowed view of the open container's current slots.
    #[must_use]
    pub fn container_item_slice(&self) -> &[RuntimeItemStack] {
        if self.ender_chest_menu {
            return self.ender_chest.slot_slice();
        }
        &self.container
    }

    /// Number of slots in the open container without cloning any items.
    #[must_use]
    pub fn container_item_count(&self) -> usize {
        self.container_item_slice().len()
    }

    /// Re-reads the container the open menu was built over.
    ///
    /// Vanilla's menu holds the `Container` itself, so every read is already
    /// current; here the caller refreshes the copy before it runs anything
    /// that reads a container slot. A list of the wrong length is refused
    /// rather than resized: `checkContainerSize` (`ChestMenu.java:51`) is the
    /// constructor's own precondition, and the menu is already built.
    pub fn refresh_container_items(&mut self, items: Vec<RuntimeItemStack>) {
        if items.len() != self.container.len() {
            return;
        }
        self.container = items;
    }

    /// `ServerPlayer.nextContainerCounter` (`ServerPlayer.java:1314-1316`).
    pub fn next_container_counter(&mut self) -> i32 {
        self.container_counter = self.container_counter % CONTAINER_COUNTER_WRAP + 1;
        self.container_counter
    }

    /// `ServerPlayer.openMenu(MenuProvider)` (`ServerPlayer.java:1319-1342`).
    #[cfg(test)]
    pub(crate) fn open_menu(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        maps: Option<&mut ServerMapStore>,
        layout: MenuLayout,
        anchor: ContainerAnchor,
        container: Vec<RuntimeItemStack>,
        has_infinite_materials: bool,
    ) -> Result<Option<(OpenedMenu, ClosedMenu)>> {
        let Some((prepared, closed)) = self.prepare_menu(
            resolver,
            maps,
            layout,
            anchor,
            container,
            has_infinite_materials,
        )?
        else {
            return Ok(None);
        };
        let initial = self.send_all_data_to_remote()?;
        Ok(Some((
            OpenedMenu {
                container_id: prepared.container_id,
                menu_type_id: prepared.menu_type_id,
                initial,
            },
            closed,
        )))
    }

    #[cfg(test)]
    pub(crate) fn prepare_menu(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        maps: Option<&mut ServerMapStore>,
        layout: MenuLayout,
        anchor: ContainerAnchor,
        container: Vec<RuntimeItemStack>,
        has_infinite_materials: bool,
    ) -> Result<Option<(PreparedMenu, ClosedMenu)>> {
        if layout.menu_type_id().is_none() {
            return Ok(None);
        }
        if container.len() != layout.container_size() {
            // `AbstractContainerMenu.checkContainerSize` (:105-110) throws
            // here, which vanilla's caller never triggers because the provider
            // and the menu agree on the row count.
            return Ok(None);
        }
        let closed = if self.container_id() != MENU_CONTAINER_ID {
            self.do_close_container(resolver, has_infinite_materials)?
        } else {
            ClosedMenu::default()
        };
        let container_id = self.next_container_counter();
        Ok(self
            .prepare_reserved_menu(resolver, maps, container_id, layout, anchor, container)?
            .map(|prepared| (prepared, closed)))
    }

    /// The World caller closes the old menu before provider admission and loot
    /// unpacking (`ServerPlayer.java:1324-1329`).
    pub(crate) fn prepare_reserved_menu(
        &mut self,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        maps: Option<&mut ServerMapStore>,
        container_id: i32,
        layout: MenuLayout,
        anchor: ContainerAnchor,
        container: Vec<RuntimeItemStack>,
    ) -> Result<Option<PreparedMenu>> {
        let prepared =
            self.prepare_container_menu(resolver, maps, container_id, layout, anchor, container)?;
        if prepared.is_some() {
            self.menu_items()?;
            self.carried().summary();
        }
        Ok(prepared)
    }

    fn prepare_container_menu(
        &mut self,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        maps: Option<&mut ServerMapStore>,
        container_id: i32,
        layout: MenuLayout,
        anchor: ContainerAnchor,
        container: Vec<RuntimeItemStack>,
    ) -> Result<Option<PreparedMenu>> {
        let Some(menu_type_id) = layout.menu_type_id() else {
            return Ok(None);
        };
        Ok(self
            .install_container_menu(resolver, maps, container_id, layout, anchor, container)?
            .then_some(PreparedMenu {
                container_id,
                menu_type_id,
            }))
    }

    pub(super) fn install_container_menu(
        &mut self,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        maps: Option<&mut ServerMapStore>,
        container_id: i32,
        layout: MenuLayout,
        anchor: ContainerAnchor,
        container: Vec<RuntimeItemStack>,
    ) -> Result<bool> {
        if container.len() != layout.container_size() {
            return Ok(false);
        }
        self.suspended_inventory_menu = Some(std::mem::replace(
            &mut self.menu,
            MenuState::new(container_id, layout),
        ));
        self.container = container;
        self.ender_chest_menu = false;
        self.container_instances.clear();
        self.rematch_crafter_preview(resolver, maps)?;
        self.container_anchor = Some(anchor);
        self.menu_epoch = self.menu_epoch.wrapping_add(1);
        self.suspend_menu_synchronizer();
        Ok(true)
    }

    /// The non-empty stacks a menu-owned container hands back when the menu
    /// is removed: the merchant payments or a crafting table's grid.
    pub fn take_menu_owned_items(&mut self) -> Vec<RuntimeItemStack> {
        if self.menu.utility.is_some() {
            return self.take_utility_inputs();
        }
        if self.is_merchant_menu() {
            return self
                .take_merchant_payments()
                .into_iter()
                .filter(|item| !item.is_empty())
                .collect();
        }
        self.take_crafting_table_grid()
    }

    /// `ServerPlayer.doCloseContainer` (`ServerPlayer.java:1405-1409`):
    /// `containerMenu.removed(this)`, `inventoryMenu.transferState(...)` and
    /// the identity swap back.
    ///
    /// With no menu open this is `InventoryMenu.removed`
    /// (InventoryMenu.java:88-94) on itself: the result slot is cleared and
    /// the crafting grid is handed back through `clearContainer`
    /// (AbstractContainerMenu.java:608-612). With one open it is
    /// `ChestMenu.removed` (`ChestMenu.java:100-103`), whose own half is
    /// `stopOpen` and which leaves the container's contents alone.
    pub fn do_close_container(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        has_infinite_materials: bool,
    ) -> Result<ClosedMenu> {
        self.close_container_items(resolver, has_infinite_materials, false)
    }

    /// The same close for a killed body (`Player.remove`, Player.java:1244-1249):
    /// `dropOrPlaceInInventory` (AbstractContainerMenu.java:598-606) and
    /// `MerchantMenu.removed` (`MerchantMenu.java:154`) drop every stack.
    pub fn close_container_for_removed_player(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> Result<ClosedMenu> {
        self.close_container_items(resolver, false, true)
    }

    fn close_container_items(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        has_infinite_materials: bool,
        removed: bool,
    ) -> Result<ClosedMenu> {
        // The furnace admission sets, the data-slot mirror and the result
        // take state belong to the menu they were attached to.
        self.clear_furnace_admission();
        self.clear_brewing_fuel();
        self.menu_data.clear();
        self.furnace_result_remove_count = 0;
        self.furnace_result_takes = 0;
        // `AbstractContainerMenu.removed` (:588-596) runs first for either
        // menu and hands the cursor back, so a click that left something
        // carried does not lose it when the menu closes.
        let mut filled = Vec::new();
        let mut dropped = Vec::new();
        let mut carried = self.menu.carried.clone();
        if !carried.is_empty() {
            if !removed {
                filled.extend(self.place_item_back_in_inventory(
                    resolver,
                    &mut carried,
                    has_infinite_materials,
                )?);
            }
            if !carried.is_empty() {
                dropped.push(carried);
            }
        }
        self.menu.carried = RuntimeItemStack::empty();
        // `MerchantMenu.removed` (`MerchantMenu.java:164-166`) and
        // `CraftingMenu.removed` (`CraftingMenu.java:98-101`) for a live player.
        // `BeaconMenu.removed` (`BeaconMenu.java:56-64`) drops the payment
        // where the other menus hand their inputs back.
        let drops_owned =
            removed || self.utility_menu_kind() == Some(super::UtilityMenuKind::Beacon);
        for mut item in self.take_menu_owned_items() {
            if !drops_owned {
                filled.extend(self.place_item_back_in_inventory(
                    resolver,
                    &mut item,
                    has_infinite_materials,
                )?);
            }
            if !item.is_empty() {
                dropped.push(item);
            }
        }
        let Some(mut inventory_menu) = self.suspended_inventory_menu.take() else {
            // `InventoryMenu.removed`'s own half (InventoryMenu.java:90-93).
            self.crafting_result = RuntimeItemStack::empty();
            for index in 0..self.crafting_grid.len() {
                let mut original =
                    std::mem::replace(&mut self.crafting_grid[index], RuntimeItemStack::empty());
                if !removed {
                    filled.extend(self.place_item_back_in_inventory(
                        resolver,
                        &mut original,
                        has_infinite_materials,
                    )?);
                }
                if !original.is_empty() {
                    dropped.push(original);
                }
            }
            return Ok(ClosedMenu {
                dropped,
                filled,
                closed_container: None,
            });
        };
        // `AbstractContainerMenu.transferState` (:785-806): every slot the two
        // menus share -- the same `Container` at the same index -- carries its
        // remote mirror across, so the inventory menu does not re-send slots
        // the client already shows. Here the shared slots are exactly the ones
        // both layouts resolve to the same `MenuSlot::Inventory`.
        let closing = std::mem::replace(
            &mut self.menu,
            MenuState::new(0, MenuLayout::PlayerInventory),
        );
        for slot in 0..inventory_menu.layout.slot_count() {
            // Only the player's `Inventory` is shared; a crafting table's grid
            // is not the 2x2 grid it shares slot indices with.
            let Some(target @ super::MenuSlot::Inventory(_)) = inventory_menu.layout.slot(slot)
            else {
                continue;
            };
            let Some(source) = (0..closing.layout.slot_count())
                .find(|other| closing.layout.slot(*other) == Some(target))
            else {
                continue;
            };
            let (Ok(into), Ok(from)) = (usize::try_from(slot), usize::try_from(source)) else {
                continue;
            };
            inventory_menu.remote_slots[into] = closing.remote_slots[from].clone();
            inventory_menu.last_slots[into] = closing.last_slots[from].clone();
        }
        self.menu_epoch = self.menu_epoch.wrapping_add(1);
        self.menu = inventory_menu;
        self.container = Vec::new();
        self.ender_chest_menu = false;
        self.container_instances.clear();
        Ok(ClosedMenu {
            dropped,
            filled,
            closed_container: self.container_anchor.take(),
        })
    }
}
