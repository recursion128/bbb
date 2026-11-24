//! The `Inventory` half: selected slot, hands, adding and picking items, slot access.

use crate::entity::item_components::enchantment_count;
use crate::world::maps::ServerMapStore;
use bbb_facts::interaction_hands::InteractionHand;
use bbb_protocol::{
    component_literal_nbt,
    data_component_types::{
        VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
        VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
    },
    equipment_slots::{EquipmentSlot, VANILLA_EQUIPMENT_SLOTS},
    item_types::{VANILLA_ITEM_STONE_ID, VANILLA_ITEM_WRITTEN_BOOK_ID},
    packets::{EditBook, ItemStackSummary as WireItemStack, WrittenBookContentSummary},
};
use bbb_registry_codec::PersistentItemStackHolderResolver;

use bbb_server_types::Result;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use super::item_rules::{
    add_patched_component, has_component, is_damaged, is_stackable, remove_patched_component,
};
use super::menu_layout::{
    addressable_slot_count, is_equippable_in_slot, item_places_a_shulker_box, player_inventory_slot,
};
use super::{
    MenuLayout, MenuSlot, MenuSlotKind, MenuState, PlayerInventoryMenu, ARMOR_SLOT_MAX_STACK_SIZE,
    CONTAINER_MAX_STACK_SIZE, INVENTORY_SIZE, MENU_CONTAINER_ID, MENU_CRAFT_SLOT_END,
    MENU_CRAFT_SLOT_START, SELECTION_SIZE, SLOT_CLICKED_OUTSIDE,
};
use bbb_server_types::item_stack::RuntimeItemStack;

impl PlayerInventoryMenu {
    pub fn empty() -> Self {
        Self {
            inventory: vec![RuntimeItemStack::empty(); addressable_slot_count()],
            crafting_grid: vec![
                RuntimeItemStack::empty();
                usize::try_from(MENU_CRAFT_SLOT_END - MENU_CRAFT_SLOT_START)
                    .expect("the crafting grid range is non-negative")
            ],
            ender_chest: super::ender_chest::EnderChestContainer::default(),
            crafting_result: RuntimeItemStack::empty(),
            crafting: super::crafting::InventoryCrafting::default(),
            selected: 0,
            // `InventoryMenu`'s constructor is `super(null, 0, 2, 2)`
            // (InventoryMenu.java:49): no menu type, container zero.
            menu: MenuState::new(MENU_CONTAINER_ID, MenuLayout::PlayerInventory),
            suspended_inventory_menu: None,
            container: Vec::new(),
            container_anchor: None,
            ender_chest_menu: false,
            container_instances: Vec::new(),
            container_counter: 0,
            menu_epoch: 0,
            furnace_inputs: None,
            furnace_fuels: None,
            brewing_fuels: None,
            menu_data: BTreeMap::new(),
            furnace_result_remove_count: 0,
            furnace_result_takes: 0,
            brewed_potion_take: None,
            armor_change_locks: Arc::default(),
            entity_tags: crate::entity::EntityRegistryTagView::Builtin26_1,
            content_registry: Arc::clone(bbb_protocol::content_registry::ContentRegistry::vanilla()),
            equip_changes: Vec::new(),
        }
    }

    /// A stocked inventory for the tests that need one: [`Self::empty`] with a
    /// full stack in the first slot.
    pub fn fixture() -> Self {
        let mut menu = Self::empty();
        menu.inventory[0] = WireItemStack {
            item_id: Some(VANILLA_ITEM_STONE_ID),
            count: 64,
            component_patch: Default::default(),
        }
        .into();
        menu
    }

    /// The `InventoryMenu` a respawned session carries.
    ///
    /// `PlayerList.respawn` builds a whole new `ServerPlayer`
    /// (PlayerList.java:390), so both the `Inventory` and the `InventoryMenu`
    /// over it are new: the remote mirror starts uninitialised, which is what
    /// makes the next `broadcastChanges` republish every slot to a client that
    /// has just rebuilt its own from the respawn packet.
    ///
    /// `ServerPlayer.restoreFrom` fills that inventory from the old player's
    /// only when the death kept the data (ServerPlayer.java:1605-1607) --
    /// `transferInventoryXpAndScore` (`:1627-1633`) through
    /// `Inventory.replaceWith` (Inventory.java:500-506), which copies the slots
    /// and the selected one. Otherwise it stays the inventory a fresh player is
    /// built with (Player.java:174-180), whose slots are `ItemStack.EMPTY`
    /// (Inventory.java:55).
    pub fn respawned(&self, keep_inventory: bool) -> Self {
        let mut menu = Self::empty();
        menu.menu_epoch = self.menu_epoch.wrapping_add(1);
        menu.crafting = self.crafting.respawned();
        if keep_inventory {
            menu.inventory = self.inventory.clone();
            menu.selected = self.selected;
        }
        // ServerPlayer.java:1611: the original ender container crosses every death.
        menu.ender_chest = self.ender_chest.clone();
        menu.content_registry = Arc::clone(&self.content_registry);
        menu
    }

    /// `Inventory.selected`, which `ServerboundSetCarriedItemPacket` moves.
    pub fn selected_slot(&self) -> u8 {
        self.selected
    }

    pub fn set_selected_slot(&mut self, slot: u8) {
        self.selected = slot;
    }

    /// `Inventory.getSelected`.
    pub fn selected_item(&self) -> Result<WireItemStack> {
        Ok(self.selected_stack().summary().clone())
    }

    pub fn selected_stack(&self) -> &RuntimeItemStack {
        &self.inventory[usize::from(self.selected)]
    }

    /// The same slot `Inventory.getSelected` answers, to be written in place.
    ///
    /// Vanilla hands the *live* stack to the paths that damage or shrink it --
    /// `ServerPlayerGameMode.destroyBlock` takes its copy first and then calls
    /// `itemStack.mineBlock` on the original (`ServerPlayerGameMode.java:293`,
    /// `:296`) -- so the mutation has to land back in the inventory rather than
    /// on a copy. Nothing republishes the menu here and vanilla does not
    /// either: the player tick's `containerMenu.broadcastChanges` is what tells
    /// the client, which is `crate::world::ServerWorld::tick_container_menus`.
    pub fn selected_item_mut(&mut self) -> &mut RuntimeItemStack {
        &mut self.inventory[usize::from(self.selected)]
    }

    /// The stack `Player.getItemInHand` answers for `hand`.
    pub fn item_in_hand_mut(&mut self, hand: InteractionHand) -> &mut RuntimeItemStack {
        match hand {
            InteractionHand::MainHand => self.selected_item_mut(),
            InteractionHand::OffHand => {
                &mut self.inventory[player_inventory_slot(EquipmentSlot::OffHand)]
            }
        }
    }

    /// Every `EquipmentSlot` this inventory can answer for, in
    /// `EquipmentSlot.VALUES` order (`EquipmentSlot.java:23`), paired with the
    /// slot index `LivingEntity.getItemBySlot` reads.
    ///
    /// `MAINHAND` is the one slot whose index is not a constant: it is
    /// `Inventory.selected` (`Inventory.getSelected`), which is why the
    /// generated catalog carries no static slot for it and why this is a method
    /// rather than a table.
    ///
    /// The other slots use the generated `PlayerEquipment` inventory mapping.
    pub fn equipment_slots(&self) -> impl Iterator<Item = (EquipmentSlot, usize)> + '_ {
        VANILLA_EQUIPMENT_SLOTS.iter().filter_map(|entry| {
            let index = match entry.slot {
                EquipmentSlot::MainHand => usize::from(self.selected),
                _ => usize::try_from(entry.player_inventory_slot?).ok()?,
            };
            Some((entry.slot, index))
        })
    }

    /// One `Inventory` slot by index, for `Player.getItemBySlot`'s read half
    /// (`LivingEntity.getItemBySlot`).
    pub fn slot(&self, index: usize) -> Option<&RuntimeItemStack> {
        self.inventory.get(index)
    }

    /// One `Inventory` slot by index, to be written in place. See
    /// [`Self::selected_item_mut`] for why the mutation lands here rather than
    /// on a copy.
    pub fn slot_mut(&mut self, index: usize) -> Option<&mut RuntimeItemStack> {
        self.inventory.get_mut(index)
    }

    /// `AbstractContainerMenu.getCarried` (`AbstractContainerMenu.java:773-775`),
    /// the stack `Player.getSlot` answers slot 499 with
    /// (`Player.java:1705-1718`).
    pub fn carried(&self) -> &RuntimeItemStack {
        &self.menu.carried
    }

    /// `AbstractContainerMenu.setCarried` (`AbstractContainerMenu.java:769-771`).
    pub fn set_carried(&mut self, item: RuntimeItemStack) {
        self.menu.carried = item;
    }

    /// `InventoryMenu.getCraftSlots` (`InventoryMenu.java:178-180`), the 2x2
    /// grid `Player.getSlot` reads slots 500 to 503 out of
    /// (`Player.java:1720-1735`).
    pub(crate) fn crafting_slot(&self, index: usize) -> Option<RuntimeItemStack> {
        self.crafting_grid.get(index).cloned()
    }

    pub fn crafting_slots(&self) -> Vec<RuntimeItemStack> {
        self.crafting_grid.clone()
    }

    /// `craftSlots.setItem` then `inventoryMenu.slotsChanged(inventory)`
    /// (`Player.java:1731-1732`), whose server arm is
    /// `slotChangedCraftingGrid` (`InventoryMenu.java:81-83`).
    ///
    /// Answers whether the grid has that slot, which is what makes an
    /// out-of-range write a refusal rather than a silent no-op.
    pub(crate) fn set_crafting_slot(
        &mut self,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        maps: Option<&mut ServerMapStore>,
        index: usize,
        item: RuntimeItemStack,
    ) -> Result<bool> {
        let Some(slot) = self.crafting_grid.get_mut(index) else {
            return Ok(false);
        };
        *slot = item;
        self.crafting_slots_changed(resolver, maps)?;
        Ok(true)
    }

    /// `Player.getEnderChestInventory` (`Player.java:1636-1638`), the
    /// container `Player.getSlot` reads slots 200 to 226 out of
    /// (`Player.java:1741-1742`).
    pub(crate) fn ender_chest_slot(&self, index: usize) -> Option<RuntimeItemStack> {
        self.ender_chest.get(index)
    }

    pub fn ender_chest_slots(&self) -> Vec<bbb_server_types::item_stack::RuntimeItemStack> {
        self.ender_chest.slots()
    }

    /// The `set` half of the `SlotAccess` the container hands back
    /// (`Container.java:105-118`), which writes the slot and answers `true`.
    ///
    /// Answers whether the container has that slot, which is what makes an
    /// out-of-range write a refusal rather than a silent no-op.
    pub fn set_ender_chest_slot(&mut self, index: usize, item: WireItemStack) -> bool {
        self.set_ender_chest_runtime_slot(
            index,
            bbb_server_types::item_stack::RuntimeItemStack::from_summary(item),
        )
        .unwrap_or(false)
    }

    pub fn set_ender_chest_runtime_slot(
        &mut self,
        index: usize,
        item: bbb_server_types::item_stack::RuntimeItemStack,
    ) -> Result<bool> {
        self.ender_chest.set(index, item)
    }

    /// `PlayerEnderChestContainer.fromSlots`
    /// (`PlayerEnderChestContainer.java:28-38`) writes into these.
    pub fn install_ender_chest_slots(&mut self, items: &[RuntimeItemStack]) -> Result<bool> {
        let mut items = items.to_vec();
        for item in &mut items {
            super::ender_chest::limit_size(item)?;
        }
        Ok(self.ender_chest.install(items))
    }

    pub fn active_ender_chest(
        &self,
    ) -> Option<(
        bbb_server_types::save::Java26LevelKey,
        crate::block_entity::BlockEntity,
    )> {
        self.ender_chest.active_chest()
    }

    pub fn set_active_ender_chest(
        &mut self,
        chest: Option<(
            bbb_server_types::save::Java26LevelKey,
            crate::block_entity::BlockEntity,
        )>,
    ) {
        self.ender_chest.set_active_chest(chest);
    }

    /// The whole `Inventory`, in its own numbering.
    pub fn inventory_slots(&self) -> Vec<RuntimeItemStack> {
        self.inventory.to_vec()
    }

    pub fn inventory_items(&self) -> &[RuntimeItemStack] {
        &self.inventory
    }

    pub fn install_inventory_slots(&mut self, slots: &[RuntimeItemStack]) {
        for (index, item) in slots.iter().enumerate() {
            self.set_inventory_slot(index, item.clone());
        }
    }

    #[cfg(test)]
    pub(crate) fn clear_all_items(&mut self, maps: Option<&mut ServerMapStore>) -> Result<i32> {
        self.clear_matching_items(
            crate::inventory::test_item_resolver(),
            maps,
            -1,
            &mut |_| Ok(true),
        )
    }

    /// `Inventory.clearOrCountMatchingItems` (`Inventory.java:180-191`).
    pub(crate) fn clear_matching_items(
        &mut self,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        max_count: i32,
        matches: &mut impl FnMut(&RuntimeItemStack) -> Result<bool>,
    ) -> Result<i32> {
        let mut count = 0_i32;
        for index in 0..self.inventory.len() {
            let removed = clear_matching_stack(&self.inventory[index], max_count, count, matches)?;
            if removed > 0 && max_count != 0 {
                let stack =
                    self.inventory[index].with_count(self.inventory[index].count() - removed);
                self.set_inventory_slot(index, stack);
            }
            count = count.saturating_add(removed);
        }
        for index in 0..self.crafting_grid.len() {
            let removed =
                clear_matching_stack(&self.crafting_grid[index], max_count, count, matches)?;
            if removed > 0 && max_count != 0 {
                let stack = &self.crafting_grid[index];
                self.crafting_grid[index] = stack.with_count(stack.count() - removed);
                self.crafting_slots_changed(resolver, maps.as_deref_mut())?;
            }
            count = count.saturating_add(removed);
        }
        let removed = clear_matching_stack(&self.menu.carried, max_count, count, matches)?;
        if removed > 0 && max_count != 0 {
            self.set_carried(
                self.menu
                    .carried
                    .with_count(self.menu.carried.count() - removed),
            );
        }
        Ok(count.saturating_add(removed))
    }

    /// `ItemStack.consume(amount, owner)` (ItemStack.java:1082-1086) applied to
    /// the stack `Player.getItemInHand` answers for `hand`: it shrinks by
    /// `amount` unless the owner has infinite materials, and a stack that
    /// reaches zero becomes `ItemStack.EMPTY`.
    ///
    /// The infinite-materials arm is the same answer
    /// `ServerPlayerGameMode.useItemOn` reaches by its other route -- it saves
    /// the count around `ItemStack.useOn` and writes it back
    /// (`ServerPlayerGameMode.java:389-395`) -- so both of vanilla's guards on a
    /// creative placement land on this one branch rather than on two of them.
    pub fn consume_in_hand(
        &mut self,
        hand: InteractionHand,
        amount: i32,
        has_infinite_materials: bool,
    ) {
        if has_infinite_materials {
            return;
        }
        let index = match hand {
            InteractionHand::MainHand => usize::from(self.selected),
            InteractionHand::OffHand => player_inventory_slot(EquipmentSlot::OffHand),
        };
        let stack = &mut self.inventory[index];
        let count = stack.count() - amount;
        stack.set_count(count);
    }

    /// `Inventory.getItem(SLOT_OFFHAND)`, which is what
    /// `Player.getItemInHand(OFF_HAND)` reads.
    pub fn off_hand_item(&self) -> Result<WireItemStack> {
        Ok(self.off_hand_stack().summary().clone())
    }

    pub fn off_hand_stack(&self) -> &RuntimeItemStack {
        &self.inventory[player_inventory_slot(EquipmentSlot::OffHand)]
    }

    pub fn set_inventory_slot(&mut self, index: usize, item: impl Into<RuntimeItemStack>) -> bool {
        let item = item.into();
        let Some(current) = self.inventory.get_mut(index) else {
            return false;
        };
        if *current != item {
            *current = item;
        }
        self.clear_bundle_selection_for_inventory_index(index);
        true
    }

    pub fn set_item_in_hand(&mut self, hand: InteractionHand, item: impl Into<RuntimeItemStack>) {
        let index = match hand {
            InteractionHand::MainHand => usize::from(self.selected),
            InteractionHand::OffHand => player_inventory_slot(EquipmentSlot::OffHand),
        };
        self.set_inventory_slot(index, item);
    }

    /// `Inventory.isHotbarSlot` (Inventory.java:130-132).
    pub fn is_hotbar_slot(slot: usize) -> bool {
        slot < SELECTION_SIZE
    }

    /// The gate `ServerGamePacketListenerImpl.handleEditBook` (:977-989) puts
    /// in front of the edit before it hands the pages to the text filter: a
    /// hotbar slot or the off-hand, and nothing else. It is the one part of the
    /// handler that runs where the packet does — everything after it is a task.
    pub fn edit_book_slot(slot: i32) -> Option<usize> {
        let slot = usize::try_from(slot).ok()?;
        (Self::is_hotbar_slot(slot) || slot == player_inventory_slot(EquipmentSlot::OffHand))
            .then_some(slot)
    }

    /// The two writers behind `ServerGamePacketListenerImpl.handleEditBook`,
    /// `updateBookContents` (:991-997) and `signBook` (:999-1011).
    ///
    /// Both act only on a stack that has `WRITABLE_BOOK_CONTENT`. The text
    /// arrives already filtered: `TextFilter.DUMMY` (TextFilter.java:8-18) is
    /// what a server with no filter configured has, so every page comes back
    /// `FilteredText.passThrough`, which `Filterable.from`
    /// (Filterable.java:29-31) turns into a raw page with no filtered variant.
    ///
    /// Neither writer reports what it wrote — the slot reaches the client
    /// through the next per-tick `broadcastChanges`, which is why the edit is
    /// visible a tick after the request rather than inside it.
    pub fn apply_edit_book(&mut self, slot: usize, packet: &EditBook, author: &str) -> Result<()> {
        let Some(edited) = edited_book_content(self.inventory[slot].summary(), packet, author)?
        else {
            return Ok(());
        };
        if packet.title.is_some() {
            self.set_inventory_slot(slot, edited);
        } else {
            self.inventory[slot] = RuntimeItemStack::from_summary(edited);
        }
        Ok(())
    }

    /// `Inventory.findSlotMatchingItem` (Inventory.java:134-142): only the
    /// storage block, so a match never names armor or the off-hand.
    pub fn find_slot_matching_item(
        &self,
        resolver: &dyn PersistentItemStackHolderResolver,
        item: &RuntimeItemStack,
    ) -> Result<Option<usize>> {
        for slot in 0..INVENTORY_SIZE {
            if !self.inventory[slot].is_empty()
                && self.inventory[slot].same_item_same_effective_components(item, resolver)?
            {
                return Ok(Some(slot));
            }
        }
        Ok(None)
    }

    /// `Inventory.getSuitableHotbarSlot` (Inventory.java:162-178): the first
    /// empty hotbar slot from the selected one onwards, then the first
    /// unenchanted one, and the selected slot if every one of them is
    /// enchanted.
    pub fn suitable_hotbar_slot(&self) -> usize {
        let selected = usize::from(self.selected);
        for offset in 0..SELECTION_SIZE {
            let index = (selected + offset) % SELECTION_SIZE;
            if self.inventory[index].is_empty() {
                return index;
            }
        }
        for offset in 0..SELECTION_SIZE {
            let index = (selected + offset) % SELECTION_SIZE;
            if enchantment_count(&self.inventory[index]) == 0 {
                return index;
            }
        }
        selected
    }

    /// `Inventory.pickSlot` (Inventory.java:123-128): the stack changes places
    /// with whatever the chosen hotbar slot was holding.
    pub fn pick_slot(&mut self, slot: usize) {
        self.selected = u8::try_from(self.suitable_hotbar_slot()).expect("hotbar slots are small");
        let selected = usize::from(self.selected);
        self.inventory.swap(selected, slot);
        self.clear_bundle_selection_for_inventory_index(selected);
        self.clear_bundle_selection_for_inventory_index(slot);
    }

    pub(super) fn swap_inventory_slots(&mut self, first: usize, second: usize) {
        if first == second {
            return;
        }
        self.inventory.swap(first, second);
        self.clear_bundle_selection_for_inventory_index(first);
        self.clear_bundle_selection_for_inventory_index(second);
    }

    /// `Inventory.addAndPickItem` (Inventory.java:111-121): whatever the
    /// chosen hotbar slot held moves to the first free slot, if there is one,
    /// and is otherwise overwritten.
    pub fn add_and_pick_item(&mut self, item: RuntimeItemStack) {
        self.selected = u8::try_from(self.suitable_hotbar_slot()).expect("hotbar slots are small");
        let selected = usize::from(self.selected);
        if !self.inventory[selected].is_empty() {
            if let Some(free) = self.free_slot() {
                self.inventory.swap(selected, free);
            }
        }
        self.set_inventory_slot(selected, item);
    }

    /// `AbstractContainerMenu.incrementStateId` (:823-826).
    pub fn increment_state_id(&mut self) -> i32 {
        self.menu.state_id = self.menu.state_id.wrapping_add(1) & 32767;
        self.menu.state_id
    }

    /// The menu contents in slot order, which is what `sendAllDataToRemote`
    /// (:157-177) hands the synchronizer.
    pub fn menu_items(&self) -> Result<Vec<WireItemStack>> {
        (0..self.menu.layout.slot_count())
            .map(|slot| Ok(self.slot_item(slot).summary().clone()))
            .collect()
    }

    /// `Inventory.getFreeSlot` (Inventory.java:101-109), which only ever
    /// looks at the storage slots.
    pub fn free_slot(&self) -> Option<usize> {
        (0..INVENTORY_SIZE).find(|&slot| self.inventory[slot].is_empty())
    }

    /// `Inventory.hasRemainingSpaceForItem` (Inventory.java:94-99).
    pub fn has_remaining_space_for_item(
        &self,
        resolver: &dyn PersistentItemStackHolderResolver,
        slot_item: &RuntimeItemStack,
        new_item: &RuntimeItemStack,
    ) -> Result<bool> {
        Ok(slot_item.item_id().is_some()
            && slot_item.same_item_same_effective_components(new_item, resolver)?
            && is_stackable(slot_item.summary())?
            && slot_item.count() < self.container_max_stack_size(slot_item)?)
    }

    /// `Container.getMaxStackSize(ItemStack)` (Container.java:38-40).
    pub fn container_max_stack_size(&self, item: &RuntimeItemStack) -> Result<i32> {
        Ok(item.max_stack_size()?.min(CONTAINER_MAX_STACK_SIZE))
    }

    /// `Inventory.getSlotWithRemainingSpace` (Inventory.java:223-239). The held
    /// slot and the offhand are tried before the storage slots, which is why
    /// this is not a plain scan.
    pub fn slot_with_remaining_space(
        &self,
        resolver: &dyn PersistentItemStackHolderResolver,
        new_item: &RuntimeItemStack,
    ) -> Result<Option<usize>> {
        let selected = usize::from(self.selected);
        if self.has_remaining_space_for_item(resolver, &self.inventory[selected], new_item)? {
            return Ok(Some(selected));
        }
        let off_hand = player_inventory_slot(EquipmentSlot::OffHand);
        if self.has_remaining_space_for_item(resolver, &self.inventory[off_hand], new_item)? {
            return Ok(Some(off_hand));
        }
        for slot in 0..INVENTORY_SIZE {
            if self.has_remaining_space_for_item(resolver, &self.inventory[slot], new_item)? {
                return Ok(Some(slot));
            }
        }
        Ok(None)
    }

    /// `Inventory.addResource(int, ItemStack)` (Inventory.java:203-221),
    /// returning what is left of the incoming stack.
    ///
    /// `setPopTime` is left out on purpose: the pop time drives the client's
    /// own pickup animation and never reaches the wire.
    pub fn add_resource(&mut self, slot: usize, item: &mut RuntimeItemStack) -> Result<()> {
        let count = item.count();
        let empty = self.inventory[slot].is_empty();
        let capacity = if empty {
            self.container_max_stack_size(item)?
        } else {
            self.container_max_stack_size(&self.inventory[slot])?
        };
        let to_add = count.min(capacity - self.inventory[slot].count());
        if to_add <= 0 {
            return Ok(());
        }
        if empty {
            self.set_inventory_slot(slot, item.with_count(to_add));
        } else {
            let next_count = self.inventory[slot].count() + to_add;
            self.inventory[slot].set_count(next_count);
        }
        item.set_count(count - to_add);
        Ok(())
    }

    /// `Inventory.add(int, ItemStack)` (Inventory.java:250-290), which reports
    /// whether anything at all was taken.
    ///
    /// The infinite-materials branches are the ones that make a creative
    /// player's overflow disappear instead of piling up.
    pub fn add(
        &mut self,
        slot: usize,
        item: &mut RuntimeItemStack,
        has_infinite_materials: bool,
    ) -> Result<bool> {
        if item.item_id().is_none() || item.count() <= 0 {
            return Ok(false);
        }
        if is_damaged(item.summary())? {
            // A damaged stack never merges; it takes a whole slot or nothing.
            self.set_inventory_slot(slot, std::mem::replace(item, RuntimeItemStack::empty()));
            return Ok(true);
        }
        let mut last_count = item.count();
        loop {
            self.add_resource(slot, item)?;
            if item.item_id().is_none() || item.count() <= 0 || item.count() >= last_count {
                break;
            }
            last_count = item.count();
        }
        if item.count() == last_count && has_infinite_materials {
            *item = RuntimeItemStack::empty();
            return Ok(true);
        }
        Ok(item.count() < last_count)
    }

    /// `Inventory.placeItemBackInInventory` (Inventory.java:307-324), which is
    /// the one vanilla path that sends `ClientboundSetPlayerInventoryPacket`.
    ///
    /// Returns the slots that took part of the stack, in the order they were
    /// filled, and leaves whatever no slot would take in `item` — vanilla drops
    /// that as an item entity, which this world has no room for yet.
    pub fn place_item_back_in_inventory(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        item: &mut RuntimeItemStack,
        has_infinite_materials: bool,
    ) -> Result<Vec<usize>> {
        let mut filled = Vec::new();
        while item.item_id().is_some() && item.count() > 0 {
            let Some(slot) = self
                .slot_with_remaining_space(resolver, item)?
                .or_else(|| self.free_slot())
            else {
                break;
            };
            let space = item.max_stack_size()? - self.inventory[slot].count();
            let mut split = item.with_count(item.count().min(space));
            if split.count() <= 0 {
                // Vanilla splits off a non-positive amount here and loops
                // forever; the states that reach it need a slot holding more
                // than the item's own maximum, which no write this server
                // accepts can produce. Stopping turns a hang no client should
                // be able to cause into a stack that simply did not fit.
                break;
            }
            item.set_count(item.count() - (split.count()));
            if self.add(slot, &mut split, has_infinite_materials)? {
                filled.push(slot);
            }
        }
        Ok(filled)
    }

    /// `Slot.setByPlayer` (Slot.java:56-67) over a menu slot id.
    pub(crate) fn set_by_player(
        &mut self,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        maps: Option<&mut ServerMapStore>,
        slot: i16,
        item: RuntimeItemStack,
    ) -> Result<bool> {
        self.set_by_player_value(resolver, maps, slot, item)
    }

    pub(super) fn slot_value(
        &self,
        slot: i16,
    ) -> Option<bbb_server_types::item_stack::RuntimeItemStack> {
        match self.menu.layout.slot(slot)? {
            MenuSlot::Inventory(index) => Some(self.inventory[index].clone()),
            MenuSlot::Container(index) if self.ender_chest_menu => self.ender_chest.get(index),
            MenuSlot::Container(index) => Some(self.container[index].clone()),
            MenuSlot::CraftingGrid(index) => Some(self.craft_grid()[index].clone()),
            MenuSlot::CraftingResult => Some(self.craft_result().clone()),
            _ => None,
        }
    }

    pub(super) fn set_by_player_value(
        &mut self,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        maps: Option<&mut ServerMapStore>,
        slot: i16,
        item: bbb_server_types::item_stack::RuntimeItemStack,
    ) -> Result<bool> {
        self.set_by_player_with_previous(resolver, maps, slot, item, None)
    }

    /// `Slot.setByPlayer(stack, previous)` (Slot.java:60-62): `previous`
    /// defaults to the slot's current item.
    pub(super) fn set_by_player_with_previous(
        &mut self,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        maps: Option<&mut ServerMapStore>,
        slot: i16,
        item: bbb_server_types::item_stack::RuntimeItemStack,
        previous: Option<RuntimeItemStack>,
    ) -> Result<bool> {
        match self.menu.layout.slot(slot) {
            Some(MenuSlot::CraftingResult) => *self.craft_result_mut() = item,
            Some(MenuSlot::CrafterPreview) => return Ok(false),
            Some(MenuSlot::CraftingGrid(index)) => {
                self.craft_grid_mut()[index] = item;
                self.open_crafting_slots_changed(resolver, maps)?;
            }
            Some(MenuSlot::Inventory(index)) => {
                if let Some(slot) = equipment_slot_of_inventory_menu_slot(index) {
                    self.equip_changes.push(super::EquipChange {
                        slot,
                        old: previous.unwrap_or_else(|| self.inventory[index].clone()),
                        new: item.clone(),
                    });
                }
                self.set_inventory_slot(index, item);
            }
            Some(MenuSlot::Container(index)) => {
                if self.ender_chest_menu {
                    self.ender_chest.set(index, item.clone())?;
                }
                self.container[index] = item;
                self.menu_container_changed(resolver, index)?;
            }
            None => return Ok(false),
        }
        self.clear_bundle_selection(slot);
        Ok(true)
    }

    pub fn take_equip_changes(&mut self) -> Vec<super::EquipChange> {
        std::mem::take(&mut self.equip_changes)
    }

    /// `Slot.getItem` (Slot.java:48-49).
    pub fn slot_item(&self, slot: i16) -> std::borrow::Cow<'_, RuntimeItemStack> {
        use std::borrow::Cow;
        match self
            .menu
            .layout
            .slot(slot)
            .expect("callers check the slot index first")
        {
            MenuSlot::CraftingResult => Cow::Borrowed(self.craft_result()),
            MenuSlot::CrafterPreview => {
                Cow::Borrowed(&self.menu.crafter.as_ref().expect("crafter layout").preview)
            }
            MenuSlot::CraftingGrid(index) => Cow::Borrowed(&self.craft_grid()[index]),
            MenuSlot::Inventory(index) => Cow::Borrowed(&self.inventory[index]),
            MenuSlot::Container(index) if self.ender_chest_menu => Cow::Borrowed(
                self.ender_chest
                    .slot_ref(index)
                    .expect("personal menu slot"),
            ),
            MenuSlot::Container(index) => Cow::Borrowed(&self.container[index]),
        }
    }

    pub(super) fn set_slot_item_in_place(
        &mut self,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        slot: i16,
        item: RuntimeItemStack,
    ) -> Result<bool> {
        let changed = self.addressable(slot) && self.slot_item(slot).as_ref() != &item;
        let written = if let Some(MenuSlot::CraftingGrid(index)) = self.menu.layout.slot(slot) {
            self.craft_grid_mut()[index] = item;
            self.open_crafting_slots_changed(resolver, maps.as_deref_mut())?;
            true
        } else if let Some(MenuSlot::CraftingResult) = self.menu.layout.slot(slot) {
            *self.craft_result_mut() = item;
            true
        } else if let Some(MenuSlot::Inventory(index)) = self.menu.layout.slot(slot) {
            self.inventory[index] = item;
            true
        } else if let Some(MenuSlot::Container(index)) = self.menu.layout.slot(slot) {
            if self.ender_chest_menu {
                self.ender_chest.set_unchecked(index, item);
            } else {
                self.container[index] = item;
            }
            self.menu_container_changed(resolver, index)?;
            true
        } else {
            return self.set_by_player(resolver, maps, slot, item);
        };
        if changed {
            self.clear_bundle_selection(slot);
        }
        Ok(written)
    }

    pub(super) fn finish_slot_move(
        &mut self,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        slot: i16,
        item: RuntimeItemStack,
    ) -> Result<()> {
        let empty = item.is_empty();
        if let Some(clicked) = self.slot_value(slot) {
            self.set_slot_item_in_place(resolver, maps.as_deref_mut(), slot, item)?;
            if empty {
                // `InventoryMenu.quickMoveStack` (InventoryMenu.java:144-148).
                self.set_by_player_with_previous(
                    resolver,
                    maps.as_deref_mut(),
                    slot,
                    RuntimeItemStack::empty(),
                    Some(clicked),
                )?;
            } else if self.is_merchant_menu() {
                self.update_sell_item(resolver)?;
            }
        } else {
            self.set_by_player(resolver, maps, slot, item)?;
        }
        Ok(())
    }

    /// `Container.setChanged` of a menu-owned container slot: the merchant's
    /// `MerchantContainer.setItem` (`MerchantContainer.java:70-76`) or a
    /// utility menu's `slotsChanged`.
    fn menu_container_changed(
        &mut self,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        index: usize,
    ) -> Result<()> {
        if self.is_merchant_menu() && index < 2 {
            return self.update_sell_item(resolver);
        }
        if self.menu.utility.is_some() {
            return self.utility_container_changed(index);
        }
        Ok(())
    }

    /// `AbstractContainerMenu.isValidSlotIndex` (:119-121). The outside-click
    /// and no-slot sentinels are valid indexes that address no slot.
    pub fn is_valid_slot_index(&self, slot: i16) -> bool {
        slot == -1
            || slot == SLOT_CLICKED_OUTSIDE
            || (0..self.menu.layout.slot_count()).contains(&slot)
    }

    /// Whether a slot id addresses a slot of the open menu at all, i.e.
    /// whether `this.slots.get(slotIndex)` would answer one.
    pub fn addressable(&self, slot: i16) -> bool {
        self.menu.layout.slot(slot).is_some()
    }

    /// `Slot.getMaxStackSize()` (Slot.java:73-75) with `ArmorSlot`'s override.
    pub fn slot_max_stack_size(&self, slot: i16) -> i32 {
        match self.menu.layout.slot_kind(slot) {
            MenuSlotKind::Armor(_) | MenuSlotKind::MountEquipment(_) => ARMOR_SLOT_MAX_STACK_SIZE,
            MenuSlotKind::LecternBook => 1,
            // `BrewingStandMenu.PotionSlot.getMaxStackSize`
            // (`BrewingStandMenu.java:170-172`) is one for every item.
            MenuSlotKind::BrewingPotion => 1,
            MenuSlotKind::UtilityInput => self
                .utility_max_stack_size(slot)
                .unwrap_or(CONTAINER_MAX_STACK_SIZE),
            _ => CONTAINER_MAX_STACK_SIZE,
        }
    }

    /// `Slot.getMaxStackSize(ItemStack)` (Slot.java:77-79) with
    /// `FurnaceFuelSlot`'s override (`FurnaceFuelSlot.java:22-25`), which caps
    /// a bucket at one whatever its own stack size is.
    pub fn slot_max_stack_size_for(&self, slot: i16, item: &RuntimeItemStack) -> Result<i32> {
        let max = if self.menu.layout.slot_kind(slot) == MenuSlotKind::FurnaceFuel
            && item.item_id() == Some(bbb_protocol::item_types::VANILLA_ITEM_BUCKET_ID)
        {
            1
        } else {
            self.slot_max_stack_size(slot)
        };
        Ok(max.min(item.max_stack_size()?))
    }

    /// `Slot.mayPlace` (Slot.java:44-45) with the `ResultSlot` and `ArmorSlot`
    /// overrides.
    pub fn slot_may_place(&self, slot: i16, item: &RuntimeItemStack) -> Result<bool> {
        match self.menu.layout.slot_kind(slot) {
            MenuSlotKind::CraftingResult => Ok(false),
            MenuSlotKind::CrafterPreview => Ok(false),
            MenuSlotKind::CrafterInput => {
                Ok(!self.menu.crafter.as_ref().expect("crafter layout").disabled[slot as usize])
            }
            // `ArmorSlot.mayPlace` over `LivingEntity.isEquippableInSlot`
            // (`LivingEntity.java:3821-3826`), `allowed_entities` included.
            MenuSlotKind::Armor(equipment) => Ok(is_equippable_in_slot(item.summary(), equipment)?
                && crate::entity::living::valid_equipment::equippable(item)
                    .flatten()
                    .is_none_or(|equippable| {
                        crate::entity::living::valid_equipment::can_be_equipped_by(
                            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
                            equippable,
                            &self.entity_tags,
                        ) == Some(true)
                    })),
            MenuSlotKind::Plain => Ok(true),
            MenuSlotKind::ShulkerBox => Ok(!item_places_a_shulker_box(item.summary())),
            MenuSlotKind::LecternBook => Ok(false),
            // `FurnaceFuelSlot.mayPlace` (`FurnaceFuelSlot.java:16-18`): a
            // fuel, or a bucket.
            MenuSlotKind::FurnaceFuel => Ok(self.furnace_fuel_allows(item)),
            // `FurnaceResultSlot.mayPlace` (`FurnaceResultSlot.java:21-24`).
            MenuSlotKind::FurnaceResult => Ok(false),
            // `AbstractFurnaceMenu`'s input slot is a plain `Slot`.
            MenuSlotKind::FurnaceInput => Ok(true),
            // `BrewingStandMenu.PotionSlot.mayPlaceItem`
            // (`BrewingStandMenu.java:184-186`).
            MenuSlotKind::BrewingPotion => Ok(super::brewing_potion_item(item.summary())),
            // `BrewingStandMenu.IngredientsSlot.mayPlace` (`:153-156`) over
            // `PotionBrewing.isIngredient` (`PotionBrewing.java:31-33`).
            MenuSlotKind::BrewingIngredient => self.brewing_ingredient_allows(item),
            // `BrewingStandMenu.FuelSlot.mayPlaceItem` (`:135-137`) over the
            // live `minecraft:brewing_fuel` tag.
            MenuSlotKind::BrewingFuel => Ok(self.brewing_fuel_item(item)),
            // `MerchantResultSlot.mayPlace` (`MerchantResultSlot.java:23-25`).
            MenuSlotKind::MerchantResult => Ok(false),
            MenuSlotKind::UtilityInput => self.utility_may_place(slot, item),
            MenuSlotKind::UtilityResult => Ok(false),
            MenuSlotKind::MountEquipment(equipment) => Ok(self.mount_may_place(equipment, item)),
        }
    }

    /// `PotionBrewing.isIngredient` (`PotionBrewing.java:31-33`), which the
    /// brewing stand's ingredient slot admits.
    pub(super) fn brewing_ingredient_allows(&self, item: &RuntimeItemStack) -> Result<bool> {
        Ok(crate::potion_brewing::PotionBrewing::vanilla().is_ingredient(item.summary()))
    }

    /// `BrewingStandMenu.FuelSlot.mayPlaceItem` (`BrewingStandMenu.java:135-137`)
    /// over the attached `minecraft:brewing_fuel` members.
    pub(super) fn brewing_fuel_item(&self, item: &RuntimeItemStack) -> bool {
        item.item_id().is_some_and(|item_id| {
            self.brewing_fuels
                .as_ref()
                .is_some_and(|fuels| fuels.contains(&item_id))
        })
    }

    /// The `minecraft:brewing_fuel` members
    /// (`BrewingStandMenu.java:135-137`), attached when the menu opens.
    pub fn attach_brewing_fuel(&mut self, fuels: Arc<BTreeSet<i32>>) {
        self.brewing_fuels = Some(fuels);
    }

    pub(super) fn clear_brewing_fuel(&mut self) {
        self.brewing_fuels = None;
    }

    /// `FurnaceFuelSlot.mayPlace` (`FurnaceFuelSlot.java:16-18`) over the
    /// attached fuel set: `isFuel(item) || isBucket(item)`, where `isBucket`
    /// (`:27-29`) is the empty bucket alone.
    pub(super) fn furnace_fuel_allows(&self, item: &RuntimeItemStack) -> bool {
        self.furnace_fuel_item(item)
            || item.item_id() == Some(bbb_protocol::item_types::VANILLA_ITEM_BUCKET_ID)
    }

    pub(super) fn furnace_fuel_item(&self, item: &RuntimeItemStack) -> bool {
        item.item_id().is_some_and(|item_id| {
            self.furnace_fuels
                .as_ref()
                .is_some_and(|fuels| fuels.contains(&item_id))
        })
    }

    pub(super) fn furnace_input_item(&self, item: &RuntimeItemStack) -> bool {
        item.item_id().is_some_and(|item_id| {
            self.furnace_inputs
                .as_ref()
                .is_some_and(|inputs| inputs.contains(&item_id))
        })
    }

    /// The furnace menu's two datapack-driven admission sets
    /// (`AbstractFurnaceMenu.java:140-146`), attached when the menu opens.
    pub fn attach_furnace_admission(
        &mut self,
        inputs: Arc<BTreeSet<i32>>,
        fuels: Arc<BTreeSet<i32>>,
    ) {
        self.furnace_inputs = Some(inputs);
        self.furnace_fuels = Some(fuels);
    }

    pub(super) fn clear_furnace_admission(&mut self) {
        self.furnace_inputs = None;
        self.furnace_fuels = None;
    }

    /// `AbstractContainerMenu.synchronizeDataSlotToRemote` (`:262-270`): one
    /// data value, answered with whether the client has to be told. The value
    /// is remembered either way.
    pub fn menu_data_differs(&mut self, id: i16, value: i16) -> bool {
        self.menu_data.insert(id, value) != Some(value)
    }

    /// The `sendAllDataToRemote` half (`:157-178`), which resets the mirror to
    /// the current values and sends all of them.
    pub fn set_menu_data(&mut self, id: i16, value: i16) {
        self.menu_data.insert(id, value);
    }

    /// `FurnaceResultSlot.onTake` / `onQuickCraft` (`:36-49`), whose
    /// `checkTakeAchievements` (`:50-58`) the world runs.
    pub(super) fn note_furnace_result_take(&mut self) {
        self.furnace_result_takes += 1;
    }

    /// The item in a furnace menu's result slot, which every take from it
    /// awards as `ITEM_CRAFTED` (`FurnaceResultSlot.checkTakeAchievements`).
    pub fn furnace_result_item_id(&self) -> Option<i32> {
        (self.menu.layout.slot_kind(2) == MenuSlotKind::FurnaceResult)
            .then(|| self.slot_item(2).item_id())
            .flatten()
    }

    /// The furnace result takes a click completed, if any: how many
    /// `checkTakeAchievements` runs it reached, and the `removeCount` they
    /// award between them. `None` leaves the state alone.
    pub fn take_furnace_result_take(&mut self) -> Option<(i32, i32)> {
        if self.furnace_result_takes == 0 {
            return None;
        }
        Some((
            std::mem::take(&mut self.furnace_result_takes),
            std::mem::take(&mut self.furnace_result_remove_count),
        ))
    }

    // BrewingStandMenu.java:175-184 reads the taken stack's base potion, regardless of its item.
    pub(super) fn note_brewed_potion_take(&mut self, slot: i16, taken: &RuntimeItemStack) {
        if self.menu.layout.slot_kind(slot) == MenuSlotKind::BrewingPotion {
            self.brewed_potion_take = taken.summary().component_patch.potion_contents.potion_id;
        }
    }

    pub fn take_brewed_potion_take(&mut self) -> Option<i32> {
        self.brewed_potion_take.take()
    }

    pub(crate) fn set_armor_change_locks(&mut self, locks: Arc<BTreeSet<i32>>) {
        self.armor_change_locks = locks;
    }

    pub(crate) fn set_entity_tags(&mut self, tags: crate::entity::EntityRegistryTagView) {
        self.entity_tags = tags;
    }

    pub(crate) fn set_content_registry(
        &mut self,
        content_registry: Arc<bbb_protocol::content_registry::ContentRegistry>,
    ) {
        self.content_registry = content_registry;
    }

    /// `Slot.mayPickup` (Slot.java:89-91) with `ArmorSlot.mayPickup`
    /// (ArmorSlot.java:55-60).
    pub fn slot_may_pickup(&self, slot: i16, has_infinite_materials: bool) -> bool {
        match self.menu.layout.slot_kind(slot) {
            MenuSlotKind::CrafterPreview => false,
            MenuSlotKind::UtilityResult => self.utility_result_may_pickup(has_infinite_materials),
            MenuSlotKind::Armor(_) | MenuSlotKind::MountEquipment(_) => {
                has_infinite_materials || !self.slot_has_armor_change_lock(slot)
            }
            _ => true,
        }
    }

    fn slot_has_armor_change_lock(&self, slot: i16) -> bool {
        let item = self.slot_item(slot);
        let patch = &item.summary().component_patch;
        !item.is_empty()
            && patch.added_type_ids.contains(
                &bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
            )
            && patch
                .enchantments
                .iter()
                .any(|entry| self.armor_change_locks.contains(&entry.holder_id))
    }

    /// `Slot.allowModification` (Slot.java:151-153).
    pub fn slot_allows_modification(&self, slot: i16, creative: bool) -> Result<bool> {
        Ok(self.slot_may_pickup(slot, creative)
            && self.slot_may_place(slot, &self.slot_item(slot))?)
    }

    /// `Slot.remove` (Slot.java:85-87) over `Container.removeItem`
    /// (Inventory.java:331-345), which splits the amount off in place.
    ///
    /// The result slot is the exception: `ResultSlot.remove`
    /// (`ResultSlot.java:30-36`) counts what was asked for and then delegates
    /// to `ResultContainer.removeItem` (`ResultContainer.java:37-39`), which
    /// is `ContainerHelper.takeItem` (`ContainerHelper.java:17-19`) and
    /// ignores the amount outright. So a right-click that asks for half a
    /// result is handed all of it, and only the crafted stat sees the half.
    pub(crate) fn slot_remove(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        slot: i16,
        amount: i32,
    ) -> Result<RuntimeItemStack> {
        if self.menu.layout.slot_kind(slot) == MenuSlotKind::CrafterPreview {
            return Ok(RuntimeItemStack::empty());
        }
        if self.slot_item(slot).is_empty() {
            return Ok(RuntimeItemStack::empty());
        }
        if matches!(self.menu.layout.slot(slot), Some(MenuSlot::CraftingResult)) {
            let taken = std::mem::replace(self.craft_result_mut(), RuntimeItemStack::empty());
            self.note_result_remove_count(amount.min(taken.count()));
            return Self::post_process_crafting_result(maps.as_deref_mut(), resolver, taken);
        }
        if self.menu.layout.slot_kind(slot) == MenuSlotKind::MerchantResult {
            return Ok(self.merchant_result_remove(amount));
        }
        if self.menu.layout.slot_kind(slot) == MenuSlotKind::UtilityResult {
            let taken = self.utility_result_remove(slot, amount);
            if self.utility_menu_kind()
                == Some(super::utility_menu::UtilityMenuKind::CartographyTable)
            {
                return Self::post_process_crafting_result(maps.as_deref_mut(), resolver, taken);
            }
            return Ok(taken);
        }
        if amount <= 0 {
            return Ok(RuntimeItemStack::empty());
        }
        let mut current = self.slot_item(slot).into_owned();
        let taken = current.split(amount);
        if self.menu.layout.slot_kind(slot) == MenuSlotKind::FurnaceResult {
            // `FurnaceResultSlot.remove` (`FurnaceResultSlot.java:27-33`):
            // `removeCount += min(amount, getCount())`.
            self.furnace_result_remove_count += taken.count();
        }
        self.set_slot_item_in_place(resolver, maps, slot, current)?;
        Ok(taken)
    }

    /// `Slot.tryRemove` (Slot.java:97-114).
    pub(crate) fn slot_try_remove(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        slot: i16,
        amount: i32,
        max_amount: i32,
        creative: bool,
    ) -> Result<Option<RuntimeItemStack>> {
        if !self.slot_may_pickup(slot, creative) {
            return Ok(None);
        }
        if !self.slot_allows_modification(slot, creative)?
            && max_amount < self.slot_item(slot).count()
        {
            return Ok(None);
        }
        let result =
            self.slot_remove(resolver, maps.as_deref_mut(), slot, amount.min(max_amount))?;
        if result.is_empty() {
            return Ok(None);
        }
        if self.slot_item(slot).is_empty() {
            self.set_by_player_with_previous(
                resolver,
                maps,
                slot,
                RuntimeItemStack::empty(),
                Some(result.clone()),
            )?;
        }
        Ok(Some(result))
    }

    /// `Slot.safeTake` (Slot.java:116-122). Result takes run `ResultSlot.onTake`.
    pub(crate) fn slot_safe_take(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        slot: i16,
        amount: i32,
        max_amount: i32,
        creative: bool,
    ) -> Result<RuntimeItemStack> {
        let taken = self
            .slot_try_remove(
                resolver,
                maps.as_deref_mut(),
                slot,
                amount,
                max_amount,
                creative,
            )?
            .unwrap_or_else(RuntimeItemStack::empty);
        if taken.is_empty() {
            return Ok(taken);
        }
        self.note_brewed_potion_take(slot, &taken);
        match self.menu.layout.slot(slot) {
            Some(MenuSlot::CraftingResult) => {
                self.on_take_crafting_result(resolver, maps, &taken, creative)?
            }
            // `FurnaceResultSlot.onTake` (`FurnaceResultSlot.java:36-39`) runs
            // `checkTakeAchievements`, which is the world's half.
            _ if self.menu.layout.slot_kind(slot) == MenuSlotKind::FurnaceResult => {
                self.note_furnace_result_take()
            }
            _ if self.menu.layout.slot_kind(slot) == MenuSlotKind::MerchantResult => {
                self.on_take_merchant_result(resolver, &taken)?
            }
            _ if self.menu.layout.slot_kind(slot) == MenuSlotKind::UtilityResult => {
                self.on_take_utility_result(&taken)?
            }
            // `Slot.onTake` (`Slot.java:40-42`) ends in `setChanged`, which
            // `MerchantContainer.setChanged` turns into `updateSellItem`.
            Some(MenuSlot::Container(_)) if self.is_merchant_menu() => {
                self.update_sell_item(resolver)?
            }
            _ => {}
        }
        Ok(taken)
    }

    /// `Slot.safeInsert` (Slot.java:129-149), returning what did not fit.
    pub(crate) fn slot_safe_insert(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        slot: i16,
        mut input: RuntimeItemStack,
        amount: i32,
    ) -> Result<RuntimeItemStack> {
        if input.is_empty() || !self.slot_may_place(slot, &input)? {
            return Ok(input);
        }
        let held = self.slot_item(slot).into_owned();
        let transferable = amount
            .min(input.count())
            .min(self.slot_max_stack_size_for(slot, &input)? - held.count());
        if transferable <= 0 {
            return Ok(input);
        }
        if held.is_empty() {
            let moved = input.split(transferable);
            self.set_by_player(resolver, maps.as_deref_mut(), slot, moved)?;
        } else if held.same_item_same_effective_components(&input, resolver)? {
            input.set_count(input.count() - (transferable));
            self.set_slot_item_in_place(
                resolver,
                maps,
                slot,
                held.with_count(held.count() + transferable),
            )?;
        }
        Ok(input)
    }
}

/// `ContainerHelper.clearOrCountMatchingItems` (`ContainerHelper.java:66-80`).
fn clear_matching_stack(
    stack: &RuntimeItemStack,
    max_count: i32,
    count: i32,
    matches: &mut impl FnMut(&RuntimeItemStack) -> Result<bool>,
) -> Result<i32> {
    if stack.is_empty() || !matches(stack)? {
        return Ok(0);
    }
    Ok(if max_count <= 0 {
        stack.count()
    } else {
        stack.count().min(max_count.saturating_sub(count))
    })
}

fn edited_book_content(
    item: &WireItemStack,
    packet: &EditBook,
    author: &str,
) -> Result<Option<WireItemStack>> {
    if !has_component(item, VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID)? {
        return Ok(None);
    }
    let mut edited = item.clone();
    let item = &mut edited;
    match &packet.title {
        // `signBook` (:999-1011): the stack becomes a written book through
        // `ItemStack.transmuteCopy`, which keeps the patch and changes the
        // item. `remove` then drops the writable content outright rather
        // than marking it removed, because the written book's prototype
        // does not carry it — and `set` records the written content for
        // the same reason.
        Some(title) => {
            item.item_id = Some(VANILLA_ITEM_WRITTEN_BOOK_ID);
            remove_patched_component(item, VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID);
            item.component_patch.writable_book_pages.clear();
            item.component_patch.writable_book_page_filters.clear();
            item.component_patch.written_book = Some(WrittenBookContentSummary {
                title: title.as_str().into(),
                title_filter: None,
                author: author.into(),
                // `handleEditBook` signs at generation 0.
                generation: 0,
                pages: packet.pages.clone(),
                page_filters: vec![None; packet.pages.len()],
                resolved: true,
            });
            // `WrittenBookContent` pages are components, and `signBook`
            // makes each one `Component.literal` of the filtered page.
            item.component_patch.written_book_pages_nbt = packet
                .pages
                .iter()
                .map(|page| Some(component_literal_nbt(page.clone())))
                .collect();
            item.component_patch.written_book_page_filters_nbt = vec![None; packet.pages.len()];
            add_patched_component(item, VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID);
        }
        // `updateBookContents` (:991-997): the same stack, new pages.
        None => {
            item.component_patch.writable_book_pages =
                packet.pages.iter().cloned().map(Into::into).collect();
            item.component_patch.writable_book_page_filters = vec![None; packet.pages.len()];
            add_patched_component(item, VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID);
        }
    }
    Ok(Some(edited))
}

/// The armor or offhand slot an `Inventory` index holds; only the inventory
/// menu's `ArmorSlot`s and offhand slot address those indices.
pub(super) fn equipment_slot_of_inventory_menu_slot(
    index: usize,
) -> Option<bbb_protocol::equipment_slots::EquipmentSlot> {
    bbb_protocol::equipment_slots::VANILLA_EQUIPMENT_SLOTS
        .iter()
        .find(|entry| {
            entry
                .player_inventory_slot
                .is_some_and(|slot| usize::try_from(slot).ok() == Some(index))
        })
        .map(|entry| entry.slot)
}
