//! The block menus whose slots all belong to the menu: the enchanting table,
//! anvil, grindstone, smithing table, stonecutter, loom, cartography table and
//! beacon. Each kind owns its slot rules, result and quick-move in its own
//! file; this one dispatches the shared `Slot` hooks to them.

use crate::world::maps::ServerMapStore;
use bbb_registry_codec::PersistentItemStackHolderResolver;
mod anvil;
mod beacon;
mod cartography;
mod enchanting;
mod grindstone;
mod item_edits;
mod loom;
mod smithing;
mod stonecutter;

use std::collections::BTreeMap;
use std::sync::Arc;

use bbb_protocol::menu_types::{
    VANILLA_MENU_TYPE_ANVIL_ID, VANILLA_MENU_TYPE_BEACON_ID,
    VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID, VANILLA_MENU_TYPE_ENCHANTMENT_ID,
    VANILLA_MENU_TYPE_GRINDSTONE_ID, VANILLA_MENU_TYPE_LOOM_ID, VANILLA_MENU_TYPE_SMITHING_ID,
    VANILLA_MENU_TYPE_STONECUTTER_ID,
};
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result;

use super::{MenuLayout, PlayerInventoryMenu};
use crate::enchantment_facts::EnchantmentDefinition;
use crate::registry_tags::RegistryTagSnapshot;
use crate::world::registry_runtime::PlayerItemStackResolver;

pub use enchanting::{EnchantmentOffers, ENCHANTMENT_OFFER_COUNT};
pub(crate) use item_edits::transmute as transmute_item_stack;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UtilityMenuKind {
    Enchantment,
    Anvil,
    Grindstone,
    Smithing,
    Stonecutter,
    Loom,
    CartographyTable,
    Beacon,
}

impl UtilityMenuKind {
    #[must_use]
    pub fn menu_type_id(self) -> i32 {
        match self {
            Self::Enchantment => VANILLA_MENU_TYPE_ENCHANTMENT_ID,
            Self::Anvil => VANILLA_MENU_TYPE_ANVIL_ID,
            Self::Grindstone => VANILLA_MENU_TYPE_GRINDSTONE_ID,
            Self::Smithing => VANILLA_MENU_TYPE_SMITHING_ID,
            Self::Stonecutter => VANILLA_MENU_TYPE_STONECUTTER_ID,
            Self::Loom => VANILLA_MENU_TYPE_LOOM_ID,
            Self::CartographyTable => VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
            Self::Beacon => VANILLA_MENU_TYPE_BEACON_ID,
        }
    }

    /// The menu's own slots, inputs then result, ahead of
    /// `addStandardInventorySlots`.
    #[must_use]
    pub fn container_size(self) -> usize {
        match self {
            Self::Beacon => 1,
            Self::Enchantment | Self::Stonecutter => 2,
            Self::Anvil | Self::Grindstone | Self::CartographyTable => 3,
            Self::Smithing | Self::Loom => 4,
        }
    }

    #[must_use]
    pub fn result_slot(self) -> Option<i16> {
        match self {
            Self::Enchantment | Self::Beacon => None,
            Self::Stonecutter => Some(1),
            Self::Anvil | Self::Grindstone | Self::CartographyTable => Some(2),
            Self::Smithing | Self::Loom => Some(3),
        }
    }

    /// The `DataSlot`s the menu adds, in `addDataSlot` order. The beacon's
    /// three belong to its block entity.
    fn data_slot_count(self) -> usize {
        match self {
            Self::Enchantment => 10,
            Self::Anvil | Self::Smithing | Self::Stonecutter | Self::Loom => 1,
            Self::Grindstone | Self::CartographyTable | Self::Beacon => 0,
        }
    }

    /// `canTakeItemForPickAll` overrides that exclude the result container
    /// (`SmithingMenu.java:129-131`, `StonecutterMenu.java:175-177`,
    /// `CartographyTableMenu.java:137-139`).
    #[must_use]
    pub fn result_excluded_from_pick_all(self) -> bool {
        matches!(
            self,
            Self::Smithing | Self::Stonecutter | Self::CartographyTable
        )
    }
}

/// The datapack generation a utility menu reads, captured when it opens.
pub struct UtilityMenuFacts {
    pub resolver: PlayerItemStackResolver,
    pub registry_tags: Arc<RegistryTagSnapshot>,
    /// By enchantment holder id.
    pub enchantments: Arc<Vec<EnchantmentDefinition>>,
}

impl std::fmt::Debug for UtilityMenuFacts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("UtilityMenuFacts")
    }
}

impl PartialEq for UtilityMenuFacts {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// What a filled map's saved data says about it, for the cartography table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapFacts {
    pub scale: i8,
    pub locked: bool,
}

/// The player facts a click reads, refreshed before every menu operation.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UtilityPlayerFacts {
    pub experience_level: i32,
    pub infinite_materials: bool,
    pub maps: BTreeMap<i32, MapFacts>,
}

/// The world's half of a result take, in the order the menu reached it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UtilityMenuEvent {
    /// `AnvilMenu.onTake` (`AnvilMenu.java:75-115`): the levels charged, the
    /// damage roll and the level event.
    AnvilUsed { cost: i32 },
    /// `GrindstoneMenu`'s result `onTake` (`GrindstoneMenu.java:67-77`): the
    /// experience before its random half and the level event.
    GrindstoneUsed { experience: i32 },
    /// `SmithingMenu.onTake` (`SmithingMenu.java:71-78`).
    Smithed { recipe: Option<String> },
    /// `StonecutterMenu`'s result `onTake` (`StonecutterMenu.java:62-78`).
    Stonecut { recipe: Option<String> },
    /// `LoomMenu`'s result `onTake` (`LoomMenu.java:89-104`).
    LoomUsed,
    /// `CartographyTableMenu`'s result `onTake` (`CartographyTableMenu.java:68-80`).
    CartographyUsed,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct UtilityMenuState {
    kind: UtilityMenuKind,
    facts: Option<Arc<UtilityMenuFacts>>,
    player: UtilityPlayerFacts,
    events: Vec<UtilityMenuEvent>,
    /// The menu's own `DataSlot` values.
    data: Vec<i32>,
    anvil: anvil::AnvilState,
    recipe_used: Option<String>,
    stonecutter: stonecutter::StonecutterState,
    loom_patterns: Vec<i32>,
    /// `EnchantmentMenu.slotsChanged` needs the level, so the world reruns it.
    enchantment_inputs_changed: bool,
}

impl UtilityMenuState {
    pub(super) fn new(kind: UtilityMenuKind) -> Self {
        let mut data = vec![0; kind.data_slot_count()];
        if kind == UtilityMenuKind::Enchantment {
            data[enchanting::CLUES].fill(-1);
            data[enchanting::LEVEL_CLUES].fill(-1);
        }
        Self {
            kind,
            facts: None,
            player: UtilityPlayerFacts::default(),
            events: Vec::new(),
            data,
            anvil: anvil::AnvilState::default(),
            recipe_used: None,
            stonecutter: stonecutter::StonecutterState::default(),
            loom_patterns: Vec::new(),
            enchantment_inputs_changed: false,
        }
    }
}

impl PlayerInventoryMenu {
    #[must_use]
    pub fn utility_menu_kind(&self) -> Option<UtilityMenuKind> {
        match self.menu.layout {
            MenuLayout::Utility(kind) => Some(kind),
            _ => None,
        }
    }

    fn utility(&self) -> &UtilityMenuState {
        self.menu.utility.as_ref().expect("utility layout")
    }

    fn utility_mut(&mut self) -> &mut UtilityMenuState {
        self.menu.utility.as_mut().expect("utility layout")
    }

    pub fn attach_utility_facts(&mut self, facts: Arc<UtilityMenuFacts>) {
        if let Some(state) = self.menu.utility.as_mut() {
            state.facts = Some(facts);
        }
    }

    pub fn refresh_utility_player(&mut self, player: UtilityPlayerFacts) {
        if let Some(state) = self.menu.utility.as_mut() {
            state.player = player;
        }
    }

    pub fn take_utility_events(&mut self) -> Vec<UtilityMenuEvent> {
        self.menu
            .utility
            .as_mut()
            .map(|state| std::mem::take(&mut state.events))
            .unwrap_or_default()
    }

    /// The open utility menu's own data slots, or `None` for any other menu.
    #[must_use]
    pub fn utility_menu_data(&self) -> Option<&[i32]> {
        self.menu
            .utility
            .as_ref()
            .map(|state| state.data.as_slice())
    }

    /// One menu-owned slot, read for the world's half of a menu.
    #[must_use]
    pub fn utility_slot(&self, index: usize) -> RuntimeItemStack {
        self.container.get(index).cloned().unwrap_or_default()
    }

    /// `Container.setItem` on the menu's own container, which runs the
    /// menu's `slotsChanged`.
    pub fn set_utility_slot(&mut self, index: usize, item: RuntimeItemStack) -> Result<()> {
        if index >= self.container.len() {
            return Ok(());
        }
        self.container[index] = item;
        self.utility_container_changed(index)
    }

    /// `Container.setChanged` of the menu's own container: `slotsChanged`.
    pub(super) fn utility_container_changed(&mut self, index: usize) -> Result<()> {
        let kind = self.utility().kind;
        if kind.result_slot() == i16::try_from(index).ok()
            && kind != UtilityMenuKind::CartographyTable
        {
            return Ok(());
        }
        match kind {
            UtilityMenuKind::Enchantment => {
                self.utility_mut().enchantment_inputs_changed = true;
                Ok(())
            }
            UtilityMenuKind::Anvil => self.anvil_create_result(),
            UtilityMenuKind::Grindstone => self.grindstone_create_result(),
            UtilityMenuKind::Smithing => self.smithing_slots_changed(),
            UtilityMenuKind::Stonecutter => self.stonecutter_slots_changed(),
            UtilityMenuKind::Loom => self.loom_slots_changed(),
            UtilityMenuKind::CartographyTable => self.cartography_slots_changed(),
            UtilityMenuKind::Beacon => Ok(()),
        }
    }

    /// `Slot.mayPlace` of the menu's input slots.
    pub(super) fn utility_may_place(&self, slot: i16, item: &RuntimeItemStack) -> Result<bool> {
        match self.utility().kind {
            UtilityMenuKind::Enchantment => Ok(enchanting::may_place(slot, item)),
            UtilityMenuKind::Anvil | UtilityMenuKind::Stonecutter => Ok(true),
            UtilityMenuKind::Grindstone => grindstone::may_place(item),
            UtilityMenuKind::Smithing => Ok(self.smithing_may_place(slot, item)),
            UtilityMenuKind::Loom => Ok(self.loom_may_place(slot, item)),
            UtilityMenuKind::CartographyTable => cartography::may_place(slot, item),
            UtilityMenuKind::Beacon => Ok(self.beacon_payment_item(item)),
        }
    }

    /// `Slot.getMaxStackSize` of the menu's own slots.
    pub(super) fn utility_max_stack_size(&self, slot: i16) -> Option<i32> {
        match (self.utility().kind, slot) {
            // `EnchantmentMenu.java:55-60` and `BeaconMenu.PaymentSlot.getMaxStackSize`.
            (UtilityMenuKind::Enchantment | UtilityMenuKind::Beacon, 0) => Some(1),
            _ => None,
        }
    }

    /// `Slot.mayPickup` of the result slot.
    pub(super) fn utility_result_may_pickup(&self, has_infinite_materials: bool) -> bool {
        let state = self.utility();
        match state.kind {
            // `AnvilMenu.mayPickup` (`AnvilMenu.java:70-72`).
            UtilityMenuKind::Anvil => {
                let cost = state.data[0];
                (has_infinite_materials || state.player.experience_level >= cost) && cost > 0
            }
            _ => true,
        }
    }

    /// `Slot.remove` of the result slot: `ResultContainer.removeItem` hands
    /// over the whole stack (`ResultContainer.java:37-39`); the loom's output
    /// is a `SimpleContainer` that splits.
    pub(super) fn utility_result_remove(&mut self, slot: i16, amount: i32) -> RuntimeItemStack {
        let index = usize::try_from(slot).expect("result slots are non-negative");
        if self.utility().kind == UtilityMenuKind::Loom {
            return self.container[index].split(amount.max(0));
        }
        std::mem::take(&mut self.container[index])
    }

    /// The result slot's `onTake`.
    pub(super) fn on_take_utility_result(&mut self, taken: &RuntimeItemStack) -> Result<()> {
        match self.utility().kind {
            UtilityMenuKind::Anvil => self.anvil_on_take(),
            UtilityMenuKind::Grindstone => self.grindstone_on_take(),
            UtilityMenuKind::Smithing => self.smithing_on_take(taken),
            UtilityMenuKind::Stonecutter => self.stonecutter_on_take(taken),
            UtilityMenuKind::Loom => self.loom_on_take(),
            UtilityMenuKind::CartographyTable => self.cartography_on_take(),
            UtilityMenuKind::Enchantment | UtilityMenuKind::Beacon => Ok(()),
        }
    }

    pub(super) fn utility_quick_move_stack(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        slot: i16,
        dropped: &mut Vec<super::DroppedMenuStack>,
    ) -> Result<RuntimeItemStack> {
        let kind = self.utility().kind;
        if !self.addressable(slot) || self.slot_item(slot).is_empty() {
            return Ok(RuntimeItemStack::empty());
        }
        match kind {
            UtilityMenuKind::Enchantment => {
                self.enchantment_quick_move_stack(resolver, maps.as_deref_mut(), slot)
            }
            UtilityMenuKind::Anvil => self.item_combiner_quick_move_stack(
                resolver,
                maps.as_deref_mut(),
                kind,
                slot,
                |_, _| true,
            ),
            UtilityMenuKind::Smithing => self.item_combiner_quick_move_stack(
                resolver,
                maps.as_deref_mut(),
                kind,
                slot,
                Self::smithing_can_move_in,
            ),
            UtilityMenuKind::Grindstone => {
                self.grindstone_quick_move_stack(resolver, maps.as_deref_mut(), slot)
            }
            UtilityMenuKind::Stonecutter => {
                self.stonecutter_quick_move_stack(resolver, maps.as_deref_mut(), slot, dropped)
            }
            UtilityMenuKind::Loom => {
                self.loom_quick_move_stack(resolver, maps.as_deref_mut(), slot)
            }
            UtilityMenuKind::CartographyTable => {
                self.cartography_quick_move_stack(resolver, maps.as_deref_mut(), slot, dropped)
            }
            UtilityMenuKind::Beacon => self.beacon_quick_move_stack(resolver, maps, slot),
        }
    }

    /// `ItemCombinerMenu.quickMoveStack` (`ItemCombinerMenu.java:115-161`).
    fn item_combiner_quick_move_stack(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        kind: UtilityMenuKind,
        slot: i16,
        can_move_into_inputs: fn(&Self, &RuntimeItemStack) -> bool,
    ) -> Result<RuntimeItemStack> {
        let result = kind.result_slot().expect("item combiners have a result");
        let (inv_start, inv_end, use_end) = standard_ranges(kind);
        let clicked = self.slot_item(slot).into_owned();
        let mut stack = clicked.clone();
        let moved = if slot == result {
            self.move_item_stack_to(
                resolver,
                maps.as_deref_mut(),
                &mut stack,
                inv_start,
                use_end,
                true,
            )?
        } else if (0..result).contains(&slot) {
            self.move_item_stack_to(
                resolver,
                maps.as_deref_mut(),
                &mut stack,
                inv_start,
                use_end,
                false,
            )?
        } else if can_move_into_inputs(self, &stack) && (inv_start..use_end).contains(&slot) {
            self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 0, result, false)?
        } else if (inv_start..inv_end).contains(&slot) {
            self.move_item_stack_to(
                resolver,
                maps.as_deref_mut(),
                &mut stack,
                inv_end,
                use_end,
                false,
            )?
        } else if (inv_end..use_end).contains(&slot) {
            self.move_item_stack_to(
                resolver,
                maps.as_deref_mut(),
                &mut stack,
                inv_start,
                inv_end,
                false,
            )?
        } else {
            true
        };
        if !moved {
            return Ok(RuntimeItemStack::empty());
        }
        self.finish_utility_move(resolver, maps, slot, &clicked, stack)
    }

    /// The tail every utility `quickMoveStack` shares: write the remainder
    /// back, and run `onTake` when anything moved.
    fn finish_utility_move(
        &mut self,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        maps: Option<&mut ServerMapStore>,
        slot: i16,
        clicked: &RuntimeItemStack,
        stack: RuntimeItemStack,
    ) -> Result<RuntimeItemStack> {
        let remaining = stack.count();
        self.finish_slot_move(resolver, maps, slot, stack)?;
        if remaining == clicked.count() {
            return Ok(RuntimeItemStack::empty());
        }
        if Some(slot) == self.utility().kind.result_slot() {
            self.on_take_utility_result(&clicked.with_count(clicked.count() - remaining))?;
        }
        Ok(clicked.clone())
    }

    /// `clearContainer` of the menu's inputs on `removed`; the result is not
    /// handed back.
    pub(super) fn take_utility_inputs(&mut self) -> Vec<RuntimeItemStack> {
        let Some(kind) = self.utility_menu_kind() else {
            return Vec::new();
        };
        let inputs = kind
            .result_slot()
            .map_or(kind.container_size(), |slot| slot as usize);
        let taken: Vec<RuntimeItemStack> = self.container[..inputs]
            .iter_mut()
            .map(std::mem::take)
            .filter(|item| !item.is_empty())
            .collect();
        if let Some(result) = kind.result_slot() {
            self.container[result as usize] = RuntimeItemStack::empty();
        }
        taken
    }

    /// `AbstractContainerMenu.clickMenuButton` for the stonecutter and loom,
    /// whose buttons need nothing but the menu. `None` for any other menu.
    pub fn click_utility_button(&mut self, button_id: i32) -> Result<Option<bool>> {
        match self.utility_menu_kind() {
            Some(UtilityMenuKind::Stonecutter) => self.stonecutter_button(button_id).map(Some),
            Some(UtilityMenuKind::Loom) => self.loom_button(button_id).map(Some),
            _ => Ok(None),
        }
    }

    fn facts(&self) -> Option<&UtilityMenuFacts> {
        self.utility().facts.as_deref()
    }

    fn item_tag_contains(&self, tag: &str, item: &RuntimeItemStack) -> bool {
        item.item_id()
            .is_some_and(|item_id| self.crafting.tags.contains(tag, item_id))
    }
}

/// `getInventorySlotStart` .. `getUseRowEnd` of a menu with `size` own slots
/// ahead of `addStandardInventorySlots`.
fn standard_ranges(kind: UtilityMenuKind) -> (i16, i16, i16) {
    let start = kind.container_size() as i16;
    (start, start + 27, start + 36)
}
