//! `AbstractContainerMenu.clicked`: the click, quick-craft, quick-move and swap paths.

use crate::world::maps::ServerMapStore;
use bbb_facts::container_inputs::ContainerInput;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_registry_codec::PersistentItemStackHolderResolver;

use bbb_server_types::Result;

use super::item_rules::{is_damaged, is_stackable};
use super::menu_layout::{armor_menu_slot, equipment_slot_for_item};
use super::{
    DroppedMenuStack, MenuLayout, MenuSlot, MenuSlotKind, PlayerInventoryMenu, CARRIED_SLOT_SIZE,
    CHEST_MENU_ROWS, MENU_ARMOR_SLOT_END, MENU_CRAFT_SLOT_START, MENU_INV_SLOT_END,
    MENU_INV_SLOT_START, MENU_RESULT_SLOT, MENU_SHIELD_SLOT, MENU_SLOT_COUNT,
    MENU_USE_ROW_SLOT_END, MENU_USE_ROW_SLOT_START, QUICKCRAFT_HEADER_CONTINUE,
    QUICKCRAFT_HEADER_END, QUICKCRAFT_HEADER_START, QUICKCRAFT_TYPE_CHARITABLE,
    QUICKCRAFT_TYPE_CLONE, QUICKCRAFT_TYPE_GREEDY, SELECTION_SIZE_I32, SLOTS_PER_ROW,
    SLOT_CLICKED_OUTSIDE, SWAP_OFFHAND_BUTTON,
};
use bbb_server_types::item_stack::RuntimeItemStack;

#[derive(Clone, Copy)]
pub(crate) struct MenuClick {
    slot: i16,
    button: i32,
    input: ContainerInput,
    creative: bool,
}

impl PlayerInventoryMenu {
    /// `AbstractContainerMenu.clicked` (:318-332) over `doClick` (:334-546).
    ///
    /// Vanilla wraps `doClick` in a crash-report catch, so a click that runs
    /// off the end of the slot list leaves the menu wherever the failure found
    /// it and the session continues. Every place that would have thrown
    /// returns here instead, which is the same outcome without the report.
    ///
    /// Stacks the click drops into the world come back to the caller, which
    /// hands them to `Player.drop` (`Player.java:582-583`).
    pub(crate) fn clicked(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        maps: Option<&mut ServerMapStore>,
        slot: i16,
        button: i8,
        input: ContainerInput,
        creative: bool,
    ) -> Result<Vec<DroppedMenuStack>> {
        let mut dropped = Vec::new();
        self.brewed_potion_take = None;
        self.do_click(
            resolver,
            maps,
            MenuClick {
                slot,
                button: i32::from(button),
                input,
                creative,
            },
            &mut dropped,
        )?;
        // `ResultSlot.onTake` drops a remainder the inventory refused
        // (`ResultSlot.java:106-107`) from under `Slot.safeTake`, which has no
        // list to write it to.
        dropped.extend(self.take_dropped_from_craft());
        Ok(dropped)
    }

    pub(crate) fn do_click(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        click: MenuClick,
        dropped: &mut Vec<DroppedMenuStack>,
    ) -> Result<()> {
        let MenuClick {
            slot,
            button,
            input,
            creative,
        } = click;
        if input == ContainerInput::QuickCraft {
            return self.do_quick_craft(
                resolver,
                maps.as_deref_mut(),
                slot,
                button,
                creative,
                dropped,
            );
        }
        if self.menu.quickcraft_status != 0 {
            self.reset_quick_craft();
            return Ok(());
        }
        match input {
            ContainerInput::Pickup | ContainerInput::QuickMove if button == 0 || button == 1 => {
                self.do_pickup_or_quick_move(resolver, maps.as_deref_mut(), click, dropped)
            }
            ContainerInput::Swap
                if (0..SELECTION_SIZE_I32).contains(&button) || button == SWAP_OFFHAND_BUTTON =>
            {
                self.do_swap(
                    resolver,
                    maps.as_deref_mut(),
                    slot,
                    button,
                    creative,
                    dropped,
                )
            }
            ContainerInput::Clone
                if creative
                    && self.menu.carried.is_empty()
                    && slot >= 0
                    && self.addressable(slot) =>
            {
                let item = self.slot_item(slot).into_owned();
                if !item.is_empty() {
                    let full = item.max_stack_size()?;
                    self.menu.carried = item.with_count(full);
                }
                Ok(())
            }
            ContainerInput::Throw
                if self.menu.carried.is_empty() && slot >= 0 && self.addressable(slot) =>
            {
                self.do_throw(
                    resolver,
                    maps.as_deref_mut(),
                    slot,
                    button,
                    creative,
                    dropped,
                )
            }
            ContainerInput::PickupAll if slot >= 0 && self.addressable(slot) => {
                self.do_pickup_all(resolver, maps, slot, button, creative)
            }
            _ => Ok(()),
        }
    }

    /// The `QUICK_CRAFT` branch of `doClick` (:336-400): the drag that spreads
    /// the carried stack across the slots it passed over.
    pub(crate) fn do_quick_craft(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        maps: Option<&mut ServerMapStore>,
        slot: i16,
        button: i32,
        creative: bool,
        dropped: &mut Vec<DroppedMenuStack>,
    ) -> Result<()> {
        let expected_status = self.menu.quickcraft_status;
        self.menu.quickcraft_status = quickcraft_header(button);
        if (expected_status != QUICKCRAFT_HEADER_CONTINUE
            || self.menu.quickcraft_status != QUICKCRAFT_HEADER_END)
            && expected_status != self.menu.quickcraft_status
        {
            self.reset_quick_craft();
            return Ok(());
        }
        if self.menu.carried.is_empty() {
            self.reset_quick_craft();
            return Ok(());
        }
        match self.menu.quickcraft_status {
            QUICKCRAFT_HEADER_START => {
                self.menu.quickcraft_type = quickcraft_type(button);
                if is_valid_quickcraft_type(self.menu.quickcraft_type, creative) {
                    self.menu.quickcraft_status = QUICKCRAFT_HEADER_CONTINUE;
                    self.menu.quickcraft_slots.clear();
                } else {
                    self.reset_quick_craft();
                }
                Ok(())
            }
            QUICKCRAFT_HEADER_CONTINUE => {
                if !self.addressable(slot) {
                    // `this.slots.get(slotIndex)` is the first statement of
                    // this branch, so the status above is all that changed.
                    return Ok(());
                }
                let carried = self.menu.carried.clone();
                if self.can_item_quick_replace(resolver, slot, &carried, true)?
                    && self.slot_may_place(slot, &carried)?
                    && (self.menu.quickcraft_type == QUICKCRAFT_TYPE_CLONE
                        || carried.count()
                            > i32::try_from(self.menu.quickcraft_slots.len())
                                .expect("the drag cannot exceed the menu"))
                {
                    self.menu.quickcraft_slots.insert(slot);
                }
                Ok(())
            }
            QUICKCRAFT_HEADER_END => {
                self.finish_quick_craft(resolver, maps, creative, dropped)?;
                self.reset_quick_craft();
                Ok(())
            }
            _ => {
                self.reset_quick_craft();
                Ok(())
            }
        }
    }

    /// The `quickcraftStatus == 2` half of the drag (:359-397).
    pub(crate) fn finish_quick_craft(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        creative: bool,
        dropped: &mut Vec<DroppedMenuStack>,
    ) -> Result<()> {
        if self.menu.quickcraft_slots.is_empty() {
            return Ok(());
        }
        if self.menu.quickcraft_slots.len() == 1 {
            let only = *self
                .menu
                .quickcraft_slots
                .iter()
                .next()
                .expect("the set holds one slot");
            let button = self.menu.quickcraft_type;
            self.reset_quick_craft();
            // Vanilla replays the drag as a plain click of the drag's own
            // type, so one dragged-over slot behaves like clicking it.
            return self.do_click(
                resolver,
                maps.as_deref_mut(),
                MenuClick {
                    slot: only,
                    button,
                    input: ContainerInput::Pickup,
                    creative,
                },
                dropped,
            );
        }
        let source = self.menu.carried.clone();
        if source.is_empty() {
            self.reset_quick_craft();
            return Ok(());
        }
        let mut remaining = source.count();
        let slots = self
            .menu
            .quickcraft_slots
            .iter()
            .copied()
            .collect::<Vec<_>>();
        let drag_size = i32::try_from(slots.len()).expect("the drag cannot exceed the menu");
        for slot in slots {
            let carried = self.menu.carried.clone();
            if !(self.can_item_quick_replace(resolver, slot, &carried, true)?
                && self.slot_may_place(slot, &carried)?
                && (self.menu.quickcraft_type == QUICKCRAFT_TYPE_CLONE
                    || carried.count() >= drag_size))
            {
                continue;
            }
            let held = self.slot_item(slot).into_owned();
            let carry = if held.is_empty() { 0 } else { held.count() };
            let max_size = source
                .max_stack_size()?
                .min(self.slot_max_stack_size_for(slot, &source)?);
            let placed = (quick_craft_place_count(drag_size, self.menu.quickcraft_type, &source)?
                + carry)
                .min(max_size);
            remaining -= placed - carry;
            self.set_by_player(
                resolver,
                maps.as_deref_mut(),
                slot,
                source.with_count(placed),
            )?;
        }
        let mut source = source;
        source.set_count(remaining);
        self.menu.carried = source;
        Ok(())
    }

    /// The `PICKUP`/`QUICK_MOVE` branch of `doClick` (:402-468).
    pub(crate) fn do_pickup_or_quick_move(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        click: MenuClick,
        dropped: &mut Vec<DroppedMenuStack>,
    ) -> Result<()> {
        let MenuClick {
            slot,
            button,
            input,
            creative,
        } = click;
        let primary = button == 0;
        if slot == SLOT_CLICKED_OUTSIDE {
            if self.menu.carried.is_empty() {
                return Ok(());
            }
            // `player.drop(this.getCarried(), true)` (:407-410).
            if primary {
                dropped.push(DroppedMenuStack {
                    stack: std::mem::replace(&mut self.menu.carried, RuntimeItemStack::empty()),
                    thrown_from_hand: true,
                });
            } else {
                dropped.push(DroppedMenuStack {
                    stack: self.menu.carried.split(1),
                    thrown_from_hand: true,
                });
            }
            return Ok(());
        }
        if slot < 0 || !self.addressable(slot) {
            return Ok(());
        }
        if input == ContainerInput::QuickMove {
            if !self.slot_may_pickup(slot, creative) {
                return Ok(());
            }
            // `quickMoveStack` reports what it started from, and vanilla keeps
            // pulling while the slot still holds the same item, so one click
            // empties a slot rather than moving a single stack out of it.
            let mut moved =
                self.quick_move_stack(resolver, maps.as_deref_mut(), slot, creative, dropped)?;
            while !moved.is_empty() && is_same_item(&self.slot_item(slot), &moved) {
                moved =
                    self.quick_move_stack(resolver, maps.as_deref_mut(), slot, creative, dropped)?;
            }
            return Ok(());
        }
        if self.try_bundle_click_override(resolver, maps.as_deref_mut(), slot, primary, creative)? {
            return Ok(());
        }
        let clicked = self.slot_item(slot).into_owned();
        let carried = self.menu.carried.clone();
        if clicked.is_empty() {
            if !carried.is_empty() {
                let amount = if primary { carried.count() } else { 1 };
                let remaining =
                    self.slot_safe_insert(resolver, maps.as_deref_mut(), slot, carried, amount)?;
                self.menu.carried = remaining;
            }
        } else if self.slot_may_pickup(slot, creative) {
            if carried.is_empty() {
                let amount = if primary {
                    clicked.count()
                } else {
                    (clicked.count() + 1) / 2
                };
                // `Slot.tryRemove` then `Slot.onTake` (`AbstractContainerMenu.java:446-450`).
                let taken = self.slot_safe_take(
                    resolver,
                    maps.as_deref_mut(),
                    slot,
                    amount,
                    CARRIED_SLOT_SIZE,
                    creative,
                )?;
                if !taken.is_empty() {
                    self.menu.carried = taken;
                }
            } else if self.slot_may_place(slot, &carried)? {
                if clicked.same_item_same_effective_components(&carried, resolver)? {
                    let amount = if primary { carried.count() } else { 1 };
                    let remaining = self.slot_safe_insert(
                        resolver,
                        maps.as_deref_mut(),
                        slot,
                        carried,
                        amount,
                    )?;
                    self.menu.carried = remaining;
                } else if carried.count() <= self.slot_max_stack_size_for(slot, &carried)? {
                    if let Some(clicked_original) = self.slot_value(slot) {
                        let carried_original = self.menu.carried.clone();
                        self.menu.carried = clicked_original;
                        self.set_by_player_value(
                            resolver,
                            maps.as_deref_mut(),
                            slot,
                            carried_original,
                        )?;
                    } else {
                        self.menu.carried = clicked;
                        self.set_by_player(resolver, maps.as_deref_mut(), slot, carried)?;
                    }
                }
            } else if clicked.same_item_same_effective_components(&carried, resolver)? {
                let room = carried.max_stack_size()? - carried.count();
                let taken =
                    self.slot_safe_take(resolver, maps, slot, clicked.count(), room, creative)?;
                if !taken.is_empty() {
                    self.menu.carried.set_count(carried.count() + taken.count());
                }
            }
        }
        Ok(())
    }

    /// The `SWAP` branch of `doClick` (:472-511).
    ///
    /// The source is an `Inventory` slot the button names, and vanilla splits
    /// it in place — the inventory shrinks as the target fills.
    pub(crate) fn do_swap(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        slot: i16,
        button: i32,
        creative: bool,
        dropped: &mut Vec<DroppedMenuStack>,
    ) -> Result<()> {
        if !self.addressable(slot) {
            return Ok(());
        }
        let source_index = usize::try_from(button).expect("the button range is non-negative");
        let mut source = self.inventory[source_index].clone();
        let target_item = self.slot_item(slot).into_owned();
        let source_original = self.inventory[source_index].clone();
        let mut target_original = self.slot_value(slot).unwrap_or_else(|| target_item.clone());
        if source.is_empty() && target_item.is_empty() {
            return Ok(());
        }
        if let Some(MenuSlot::Inventory(target_index)) = self.menu.layout.slot(slot) {
            let allowed = if source.is_empty() {
                self.slot_may_pickup(slot, creative)
            } else {
                (target_item.is_empty() || self.slot_may_pickup(slot, creative))
                    && self.slot_may_place(slot, &source)?
                    && source.count() <= self.slot_max_stack_size_for(slot, &source)?
            };
            if allowed {
                // `target.setByPlayer(source)` (:487-509) reaches the inventory
                // menu's equipment slots' `onEquipItem`.
                if let Some(equipment) =
                    super::player_inventory::equipment_slot_of_inventory_menu_slot(target_index)
                {
                    self.equip_changes.push(super::EquipChange {
                        slot: equipment,
                        old: target_item.clone(),
                        new: source.clone(),
                    });
                }
                self.swap_inventory_slots(source_index, target_index);
                return Ok(());
            }
        }
        if source.is_empty() {
            if self.slot_may_pickup(slot, creative) {
                if matches!(self.menu.layout.slot(slot), Some(MenuSlot::CraftingResult))
                    || (self.menu.layout.slot_kind(slot) == MenuSlotKind::UtilityResult
                        && self.utility_menu_kind()
                            == Some(super::utility_menu::UtilityMenuKind::CartographyTable))
                {
                    target_original = Self::post_process_crafting_result(
                        maps.as_deref_mut(),
                        resolver,
                        target_original,
                    )?;
                }
                self.set_inventory_slot(source_index, target_original);
                self.set_by_player(
                    resolver,
                    maps.as_deref_mut(),
                    slot,
                    RuntimeItemStack::empty(),
                )?;
                self.note_brewed_potion_take(slot, &target_item);
                // `target.onSwapCraft` then `target.onTake` after an
                // empty-hotbar swap (`AbstractContainerMenu.java:478-481`), so
                // `ResultSlot.onSwapCraft` (`ResultSlot.java:45-47`) counts
                // the whole result before the take awards it.
                if matches!(self.menu.layout.slot(slot), Some(MenuSlot::CraftingResult))
                    && !target_item.is_empty()
                {
                    self.note_result_remove_count(target_item.count());
                    self.on_take_crafting_result(
                        resolver,
                        maps.as_deref_mut(),
                        &target_item,
                        creative,
                    )?;
                }
                // `target.onSwapCraft` (`AbstractContainerMenu.java:479`)
                // is a no-op for `FurnaceResultSlot`, but the `onTake` beside
                // it (`:481`) still runs `checkTakeAchievements`.
                if self.menu.layout.slot_kind(slot) == MenuSlotKind::FurnaceResult
                    && !target_item.is_empty()
                {
                    self.note_furnace_result_take();
                }
                // `MerchantResultSlot` keeps `Slot.onSwapCraft`'s no-op, then
                // `onTake` trades (`AbstractContainerMenu.java:479-481`).
                if self.menu.layout.slot_kind(slot) == MenuSlotKind::MerchantResult
                    && !target_item.is_empty()
                {
                    self.on_take_merchant_result(resolver, &target_item)?;
                }
                if self.menu.layout.slot_kind(slot) == MenuSlotKind::UtilityResult
                    && !target_item.is_empty()
                {
                    self.on_take_utility_result(&target_item)?;
                }
            }
            return Ok(());
        }
        if target_item.is_empty() {
            if self.slot_may_place(slot, &source)? {
                let max_stack = self.slot_max_stack_size_for(slot, &source)?;
                if source.count() > max_stack {
                    let moved = source.split(max_stack);
                    self.set_by_player(resolver, maps.as_deref_mut(), slot, moved)?;
                    self.set_inventory_slot(source_index, source);
                } else {
                    self.set_inventory_slot(source_index, RuntimeItemStack::empty());
                    self.set_by_player_value(resolver, maps.as_deref_mut(), slot, source_original)?;
                }
            }
            return Ok(());
        }
        if !(self.slot_may_pickup(slot, creative) && self.slot_may_place(slot, &source)?) {
            return Ok(());
        }
        let max_stack = self.slot_max_stack_size_for(slot, &source)?;
        if source.count() > max_stack {
            let moved = source.split(max_stack);
            self.set_by_player(resolver, maps.as_deref_mut(), slot, moved)?;
            self.note_brewed_potion_take(slot, &target_item);
            self.set_inventory_slot(source_index, source);
            let mut displaced = target_item;
            let added = self.add_anywhere(resolver, &mut displaced, creative)?;
            target_original.set_count(displaced.count());
            if !added {
                // `player.drop(targetItemStack, true)` (:499).
                dropped.push(DroppedMenuStack {
                    stack: target_original,
                    thrown_from_hand: true,
                });
            }
        } else {
            self.set_inventory_slot(source_index, target_original);
            self.set_by_player_value(resolver, maps, slot, source_original)?;
            self.note_brewed_potion_take(slot, &target_item);
        }
        Ok(())
    }

    /// The `THROW` branch of `doClick` (:520-544). `Player.canDropItems`
    /// (Player.java:1308-1310) is unconditionally true.
    pub(crate) fn do_throw(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        slot: i16,
        button: i32,
        creative: bool,
        dropped: &mut Vec<DroppedMenuStack>,
    ) -> Result<()> {
        let amount = if button == 0 {
            1
        } else {
            self.slot_item(slot).count()
        };
        let mut thrown = self.slot_safe_take(
            resolver,
            maps.as_deref_mut(),
            slot,
            amount,
            CARRIED_SLOT_SIZE,
            creative,
        )?;
        // `player.drop(itemStack, true)` (:522, :531).
        dropped.push(DroppedMenuStack {
            stack: thrown.clone(),
            thrown_from_hand: true,
        });
        if button != 1 {
            return Ok(());
        }
        while !thrown.is_empty() && is_same_item(&self.slot_item(slot), &thrown) {
            thrown = self.slot_safe_take(
                resolver,
                maps.as_deref_mut(),
                slot,
                amount,
                CARRIED_SLOT_SIZE,
                creative,
            )?;
            dropped.push(DroppedMenuStack {
                stack: thrown.clone(),
                thrown_from_hand: true,
            });
        }
        Ok(())
    }

    /// The `PICKUP_ALL` branch of `doClick` (:546-568): a double click that
    /// gathers matching stacks onto the cursor, smallest-first.
    pub(crate) fn do_pickup_all(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        slot: i16,
        button: i32,
        creative: bool,
    ) -> Result<()> {
        let mut carried = self.menu.carried.clone();
        let clicked_has_item = !self.slot_item(slot).is_empty();
        if carried.is_empty() || (clicked_has_item && self.slot_may_pickup(slot, creative)) {
            return Ok(());
        }
        let forwards = button == 0;
        let order: Vec<i16> = if forwards {
            (0..self.menu.layout.slot_count()).collect()
        } else {
            (0..self.menu.layout.slot_count()).rev().collect()
        };
        let carried_max = carried.max_stack_size()?;
        for pass in 0..2 {
            for target in order.iter().copied() {
                if carried.count() >= carried_max {
                    break;
                }
                let held = self.slot_item(target).into_owned();
                if held.is_empty()
                    || !self.can_item_quick_replace(resolver, target, &carried, true)?
                    || !self.slot_may_pickup(target, creative)
                    || !self.can_take_item_for_pick_all(target)
                {
                    continue;
                }
                // The first pass skips full stacks so partial ones are drained
                // before whole ones are broken up.
                if pass == 0 && held.count() == held.max_stack_size()? {
                    continue;
                }
                let taken = self.slot_safe_take(
                    resolver,
                    maps.as_deref_mut(),
                    target,
                    held.count(),
                    carried_max - carried.count(),
                    creative,
                )?;
                carried.set_count(carried.count() + (taken.count()));
            }
        }
        self.menu.carried = carried;
        Ok(())
    }

    /// `AbstractContainerMenu.canTakeItemForPickAll` (:584-586) with
    /// `InventoryMenu`'s override (InventoryMenu.java:164-166).
    pub fn can_take_item_for_pick_all(&self, slot: i16) -> bool {
        self.menu.layout.can_take_item_for_pick_all(slot)
    }

    /// `AbstractContainerMenu.canItemQuickReplace` (:727-733).
    pub fn can_item_quick_replace(
        &self,
        resolver: &dyn PersistentItemStackHolderResolver,
        slot: i16,
        item: &RuntimeItemStack,
        ignore_size: bool,
    ) -> Result<bool> {
        let held = self.slot_item(slot);
        if held.is_empty() {
            return Ok(true);
        }
        if !item.same_item_same_effective_components(&held, resolver)? {
            return Ok(false);
        }
        let added = if ignore_size { 0 } else { item.count() };
        Ok(held.count() + added <= item.max_stack_size()?)
    }

    pub fn reset_quick_craft(&mut self) {
        self.menu.quickcraft_status = 0;
        self.menu.quickcraft_slots.clear();
    }

    /// `AbstractContainerMenu.quickMoveStack` (:309), dispatched over the
    /// menu the click landed in.
    pub(crate) fn quick_move_stack(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        slot: i16,
        creative: bool,
        dropped: &mut Vec<DroppedMenuStack>,
    ) -> Result<RuntimeItemStack> {
        match self.menu.layout {
            MenuLayout::PlayerInventory => self.player_inventory_quick_move_stack(
                resolver,
                maps.as_deref_mut(),
                slot,
                creative,
                dropped,
            ),
            MenuLayout::Chest { rows } => {
                self.chest_quick_move_stack(resolver, maps.as_deref_mut(), rows, slot)
            }
            MenuLayout::Generic3x3 => {
                self.chest_quick_move_stack(resolver, maps.as_deref_mut(), 1, slot)
            }
            MenuLayout::Crafter => {
                self.crafter_quick_move_stack(resolver, maps.as_deref_mut(), slot)
            }
            MenuLayout::CraftingTable => self.crafting_table_quick_move_stack(
                resolver,
                maps.as_deref_mut(),
                slot,
                creative,
                dropped,
            ),
            MenuLayout::Hopper => {
                self.container_quick_move_stack(resolver, maps.as_deref_mut(), 5, slot)
            }
            MenuLayout::ShulkerBox => {
                self.chest_quick_move_stack(resolver, maps.as_deref_mut(), CHEST_MENU_ROWS, slot)
            }
            MenuLayout::Lectern => Ok(RuntimeItemStack::empty()),
            MenuLayout::Furnace { .. } => {
                self.furnace_quick_move_stack(resolver, maps.as_deref_mut(), slot)
            }
            MenuLayout::BrewingStand => {
                self.brewing_stand_quick_move_stack(resolver, maps.as_deref_mut(), slot)
            }
            MenuLayout::Merchant => {
                self.merchant_quick_move_stack(resolver, maps.as_deref_mut(), slot, dropped)
            }
            MenuLayout::Utility(_) => {
                self.utility_quick_move_stack(resolver, maps.as_deref_mut(), slot, dropped)
            }
            MenuLayout::Mount { columns } => {
                self.mount_quick_move_stack(resolver, maps, columns, slot)
            }
        }
    }

    /// `AbstractFurnaceMenu.quickMoveStack` (`AbstractFurnaceMenu.java:92-138`):
    /// the result and the fuel go to the player rows, a player stack goes to
    /// the input slot when a cooking recipe accepts it, then to the fuel slot
    /// when it is a fuel, and otherwise changes player rows.
    fn furnace_quick_move_stack(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        slot: i16,
    ) -> Result<RuntimeItemStack> {
        let original = self.slot_item(slot).into_owned();
        if original.is_empty() {
            return Ok(RuntimeItemStack::empty());
        }
        let end = self.menu.layout.slot_count();
        let mut stack = original.clone();
        let moved = if slot == 2 {
            self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 3, end, true)?
        } else if slot != 0 && slot != 1 {
            if self.furnace_input_item(&stack) {
                self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 0, 1, false)?
            } else if self.furnace_fuel_item(&stack) {
                self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 1, 2, false)?
            } else if (3..30).contains(&slot) {
                self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 30, 39, false)?
            } else if (30..39).contains(&slot) {
                self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 3, 30, false)?
            } else {
                false
            }
        } else {
            self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 3, end, false)?
        };
        if !moved {
            return Ok(RuntimeItemStack::empty());
        }
        if slot == 2 {
            // `AbstractFurnaceMenu.quickMoveStack` (`:98-103`) calls
            // `slot.onQuickCraft(stack, clicked)`, whose `Slot.onQuickCraft`
            // (`Slot.java:24-29`) hands `FurnaceResultSlot` the moved count.
            let moved_count = original.count() - stack.count();
            if moved_count > 0 {
                self.furnace_result_remove_count += moved_count;
                self.note_furnace_result_take();
            }
        }
        self.finish_slot_move(resolver, maps, slot, stack)?;
        Ok(original)
    }

    /// `BrewingStandMenu.quickMoveStack` (`BrewingStandMenu.java:63-115`): a
    /// player stack goes to the fuel slot when it is brewing fuel, then to the
    /// ingredient slot when the table knows it, then to a bottle slot, and
    /// otherwise changes player rows; the five container slots all go to the
    /// player's rows walked backwards.
    fn brewing_stand_quick_move_stack(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        slot: i16,
    ) -> Result<RuntimeItemStack> {
        let original = self.slot_item(slot).into_owned();
        if original.is_empty() {
            return Ok(RuntimeItemStack::empty());
        }
        let mut stack = original.clone();
        // Vanilla returns early from several branches, but the stack it moved
        // out of is the slot's own object; this menu holds a copy, so the slot
        // is written back before the answer.
        let early = if !(0..=2).contains(&slot) && slot != 3 && slot != 4 {
            if self.brewing_fuel_item(&original) {
                self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 4, 5, false)?
                    || (self.brewing_ingredient_allows(&stack)?
                        && !self.move_item_stack_to(
                            resolver,
                            maps.as_deref_mut(),
                            &mut stack,
                            3,
                            4,
                            false,
                        )?)
            } else if self.brewing_ingredient_allows(&stack)? {
                !self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 3, 4, false)?
            } else if super::brewing_potion_item(original.summary()) {
                !self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 0, 3, false)?
            } else if (5..32).contains(&slot) {
                !self.move_item_stack_to(
                    resolver,
                    maps.as_deref_mut(),
                    &mut stack,
                    32,
                    41,
                    false,
                )?
            } else if (32..41).contains(&slot) {
                !self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 5, 32, false)?
            } else {
                !self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 5, 41, false)?
            }
        } else {
            !self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 5, 41, true)?
        };
        let changed = stack.count() != original.count();
        if changed {
            self.finish_slot_move(resolver, maps, slot, stack)?;
        }
        if early || !changed {
            return Ok(RuntimeItemStack::empty());
        }
        self.note_brewed_potion_take(slot, &original);
        Ok(original)
    }

    /// `ChestMenu.quickMoveStack` (`ChestMenu.java:75-97`): a chest slot goes
    /// to the player's rows walked backwards, and a player slot goes to the
    /// chest grid walked forwards.
    pub(crate) fn chest_quick_move_stack(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        maps: Option<&mut ServerMapStore>,
        rows: i16,
        slot: i16,
    ) -> Result<RuntimeItemStack> {
        self.container_quick_move_stack(resolver, maps, rows * SLOTS_PER_ROW, slot)
    }

    pub(super) fn container_quick_move_stack(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        grid: i16,
        slot: i16,
    ) -> Result<RuntimeItemStack> {
        let original = self.slot_item(slot).into_owned();
        if original.is_empty() {
            // `:78`: a slot with no item answers `ItemStack.EMPTY`.
            return Ok(RuntimeItemStack::empty());
        }
        let end = grid + 36;
        let mut stack = original.clone();
        let moved = if slot < grid {
            self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, grid, end, true)?
        } else {
            self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 0, grid, false)?
        };
        if !moved {
            // `:83`, `:86`.
            return Ok(RuntimeItemStack::empty());
        }
        // `:89-93`: an emptied source is written back through `setByPlayer`,
        // and anything left over stays where it was.
        self.finish_slot_move(resolver, maps, slot, stack)?;
        Ok(original)
    }

    /// `InventoryMenu.quickMoveStack` (InventoryMenu.java:102-161).
    pub(crate) fn player_inventory_quick_move_stack(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        slot: i16,
        creative: bool,
        dropped: &mut Vec<DroppedMenuStack>,
    ) -> Result<RuntimeItemStack> {
        let original = self.slot_item(slot).into_owned();
        if original.is_empty() {
            return Ok(RuntimeItemStack::empty());
        }
        let mut stack = original.clone();
        if slot == MENU_RESULT_SLOT
            && stack
                .summary()
                .component_patch
                .map_post_processing
                .is_some()
        {
            if !(MENU_INV_SLOT_START..MENU_SHIELD_SLOT).any(|slot| self.slot_item(slot).is_empty())
            {
                return Ok(RuntimeItemStack::empty());
            }
            stack = Self::post_process_crafting_result(maps.as_deref_mut(), resolver, stack)?;
        }
        let equipment = equipment_slot_for_item(original.summary())?;
        let armor_target = armor_menu_slot(equipment);
        let moved = if slot == MENU_RESULT_SLOT {
            self.move_item_stack_to(
                resolver,
                maps.as_deref_mut(),
                &mut stack,
                MENU_INV_SLOT_START,
                MENU_SHIELD_SLOT,
                true,
            )?
        } else if (MENU_CRAFT_SLOT_START..MENU_ARMOR_SLOT_END).contains(&slot) {
            self.move_item_stack_to(
                resolver,
                maps.as_deref_mut(),
                &mut stack,
                MENU_INV_SLOT_START,
                MENU_SHIELD_SLOT,
                false,
            )?
        } else if armor_target.is_some_and(|target| self.slot_item(target).is_empty()) {
            let target = armor_target.expect("the branch checked it");
            self.move_item_stack_to(
                resolver,
                maps.as_deref_mut(),
                &mut stack,
                target,
                target + 1,
                false,
            )?
        } else if equipment == EquipmentSlot::OffHand && self.slot_item(MENU_SHIELD_SLOT).is_empty()
        {
            self.move_item_stack_to(
                resolver,
                maps.as_deref_mut(),
                &mut stack,
                MENU_SHIELD_SLOT,
                MENU_SLOT_COUNT,
                false,
            )?
        } else if (MENU_INV_SLOT_START..MENU_INV_SLOT_END).contains(&slot) {
            self.move_item_stack_to(
                resolver,
                maps.as_deref_mut(),
                &mut stack,
                MENU_USE_ROW_SLOT_START,
                MENU_SHIELD_SLOT,
                false,
            )?
        } else if (MENU_USE_ROW_SLOT_START..MENU_USE_ROW_SLOT_END).contains(&slot) {
            self.move_item_stack_to(
                resolver,
                maps.as_deref_mut(),
                &mut stack,
                MENU_INV_SLOT_START,
                MENU_INV_SLOT_END,
                false,
            )?
        } else {
            self.move_item_stack_to(
                resolver,
                maps.as_deref_mut(),
                &mut stack,
                MENU_INV_SLOT_START,
                MENU_SHIELD_SLOT,
                false,
            )?
        };
        if !moved {
            return Ok(RuntimeItemStack::empty());
        }
        self.finish_slot_move(resolver, maps.as_deref_mut(), slot, stack.clone())?;
        if stack.count() == original.count() {
            return Ok(RuntimeItemStack::empty());
        }
        if slot == MENU_RESULT_SLOT {
            // `InventoryMenu.quickMoveStack` (`InventoryMenu.java:109-114`, `InventoryMenu.java:154-157`).
            // `Slot.onQuickCraft` (`ResultSlot.java:39-42`) counts what moved,
            // which is what the take then awards.
            let taken_count = original.count() - stack.count();
            if taken_count > 0 {
                self.note_result_remove_count(taken_count);
                self.on_take_crafting_result(
                    resolver,
                    maps,
                    &original.with_count(taken_count),
                    creative,
                )?;
            }
            // `player.drop(stack, false)` (`InventoryMenu.java:156-158`).
            dropped.push(DroppedMenuStack {
                stack,
                thrown_from_hand: false,
            });
        }
        Ok(original)
    }

    /// `AbstractContainerMenu.moveItemStackTo` (:637-696).
    pub(crate) fn move_item_stack_to(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        stack: &mut RuntimeItemStack,
        start: i16,
        end: i16,
        backwards: bool,
    ) -> Result<bool> {
        let mut changed = false;
        let order: Vec<i16> = if backwards {
            (start..end).rev().collect()
        } else {
            (start..end).collect()
        };
        if is_stackable(stack.summary())? {
            for slot in order.iter().copied() {
                if stack.is_empty() {
                    break;
                }
                let target = self.slot_item(slot).into_owned();
                if target.is_empty()
                    || !stack.same_item_same_effective_components(&target, resolver)?
                {
                    continue;
                }
                let total = target.count() + stack.count();
                let max_stack = self.slot_max_stack_size_for(slot, &target)?;
                if total <= max_stack {
                    stack.set_count(0);
                    self.set_slot_item_in_place(
                        resolver,
                        maps.as_deref_mut(),
                        slot,
                        target.with_count(total),
                    )?;
                    changed = true;
                } else if target.count() < max_stack {
                    stack.set_count(stack.count() - (max_stack - target.count()));
                    self.set_slot_item_in_place(
                        resolver,
                        maps.as_deref_mut(),
                        slot,
                        target.with_count(max_stack),
                    )?;
                    changed = true;
                }
            }
        }
        if stack.is_empty() {
            return Ok(changed);
        }
        for slot in order {
            let target = self.slot_item(slot).into_owned();
            if !target.is_empty() || !self.slot_may_place(slot, stack)? {
                continue;
            }
            let max_stack = self.slot_max_stack_size_for(slot, stack)?;
            let moved = stack.split(stack.count().min(max_stack));
            self.set_by_player(resolver, maps.as_deref_mut(), slot, moved)?;
            changed = true;
            break;
        }
        Ok(changed)
    }

    /// `Inventory.add(ItemStack)` (Inventory.java:250-252) through
    /// `addResource(ItemStack)` (:194-201), which picks its own slot.
    pub fn add_anywhere(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        item: &mut RuntimeItemStack,
        creative: bool,
    ) -> Result<bool> {
        if item.is_empty() {
            return Ok(false);
        }
        if is_damaged(item.summary())? {
            let Some(slot) = self.free_slot() else {
                if creative {
                    item.set_count(0);
                    return Ok(true);
                }
                return Ok(false);
            };
            self.set_inventory_slot(slot, std::mem::replace(item, RuntimeItemStack::empty()));
            return Ok(true);
        }
        let mut last_count = item.count();
        loop {
            let slot = self
                .slot_with_remaining_space(resolver, item)?
                .or_else(|| self.free_slot());
            match slot {
                Some(slot) => self.add_resource(slot, item)?,
                None => break,
            }
            if item.is_empty() || item.count() >= last_count {
                break;
            }
            last_count = item.count();
        }
        if item.count() == last_count && creative {
            item.set_count(0);
            return Ok(true);
        }
        Ok(item.count() < last_count)
    }
}

/// `AbstractContainerMenu.getQuickcraftHeader` / `getQuickcraftType` (:698-704).
pub fn quickcraft_header(mask: i32) -> i32 {
    mask & 3
}

pub fn quickcraft_type(mask: i32) -> i32 {
    mask >> 2 & 3
}

/// `AbstractContainerMenu.isValidQuickcraftType` (:710-716).
pub fn is_valid_quickcraft_type(kind: i32, creative: bool) -> bool {
    match kind {
        QUICKCRAFT_TYPE_CHARITABLE | QUICKCRAFT_TYPE_GREEDY => true,
        QUICKCRAFT_TYPE_CLONE => creative,
        _ => false,
    }
}

/// `AbstractContainerMenu.getQuickCraftPlaceCount` (:735-742).
pub fn quick_craft_place_count(drag_size: i32, kind: i32, item: &RuntimeItemStack) -> Result<i32> {
    Ok(match kind {
        QUICKCRAFT_TYPE_CHARITABLE => {
            // `Mth.floor((float) count / size)`; the drag only reaches here
            // with at least two slots, so the divisor is never zero.
            (item.count() as f32 / drag_size as f32).floor() as i32
        }
        QUICKCRAFT_TYPE_GREEDY => 1,
        QUICKCRAFT_TYPE_CLONE => item.max_stack_size()?,
        _ => item.count(),
    })
}

/// `ItemStack.isSameItem` (ItemStack.java:635-637): the item alone, without
/// its components.
pub fn is_same_item(a: &RuntimeItemStack, b: &RuntimeItemStack) -> bool {
    a.item_id() == b.item_id()
}
