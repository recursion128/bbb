//! `EnchantmentMenu`'s slots and data; the offers need the level, so the world
//! computes them and the enchanting itself.

use crate::world::maps::ServerMapStore;
use bbb_protocol::item_types::{
    VANILLA_ITEM_BOOK_ID, VANILLA_ITEM_ENCHANTED_BOOK_ID, VANILLA_ITEM_LAPIS_LAZULI_ID,
};
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result;

use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID, VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
};
use bbb_server_types::item_stack::rules::has_component;

use super::item_edits::{
    crafting_enchantments, enchantment_list, transmute, upgrade, with_crafting_enchantments,
};
use super::{PlayerInventoryMenu, UtilityMenuKind};

pub const ENCHANTMENT_OFFER_COUNT: usize = 3;
/// `costs`, `enchantmentSeed`, `enchantClue` and `levelClue`, in
/// `addDataSlot` order (`EnchantmentMenu.java:73-82`).
const COSTS: std::ops::Range<usize> = 0..3;
const SEED: usize = 3;
pub(super) const CLUES: std::ops::Range<usize> = 4..7;
pub(super) const LEVEL_CLUES: std::ops::Range<usize> = 7..10;

/// The three offers `slotsChanged` shows: cost, clue enchantment holder id
/// (or -1) and clue level (or -1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EnchantmentOffers {
    pub costs: [i32; ENCHANTMENT_OFFER_COUNT],
    pub clues: [i32; ENCHANTMENT_OFFER_COUNT],
    pub level_clues: [i32; ENCHANTMENT_OFFER_COUNT],
}

/// The input slots' rules (`EnchantmentMenu.java:55-71`): any item in the
/// first, lapis in the second.
pub(super) fn may_place(slot: i16, item: &RuntimeItemStack) -> bool {
    slot == 0 || item.item_id() == Some(VANILLA_ITEM_LAPIS_LAZULI_ID)
}

impl PlayerInventoryMenu {
    fn is_enchantment_menu(&self) -> bool {
        self.utility_menu_kind() == Some(UtilityMenuKind::Enchantment)
    }

    /// Whether the enchanting inputs changed since the world last asked.
    pub fn take_enchantment_inputs_changed(&mut self) -> bool {
        self.is_enchantment_menu()
            && std::mem::take(&mut self.utility_mut().enchantment_inputs_changed)
    }

    /// `enchantmentSeed.set(...)`; the client is sent the low sixteen bits
    /// `ClientboundContainerSetDataPacket` carries.
    pub fn set_enchantment_seed(&mut self, seed: i32) {
        if self.is_enchantment_menu() {
            self.utility_mut().data[SEED] = i32::from(seed as i16);
        }
    }

    pub fn enchantment_offers(&self) -> EnchantmentOffers {
        let data = &self.utility().data;
        EnchantmentOffers {
            costs: std::array::from_fn(|i| data[COSTS.start + i]),
            clues: std::array::from_fn(|i| data[CLUES.start + i]),
            level_clues: std::array::from_fn(|i| data[LEVEL_CLUES.start + i]),
        }
    }

    pub fn set_enchantment_offers(&mut self, offers: EnchantmentOffers) {
        if !self.is_enchantment_menu() {
            return;
        }
        let data = &mut self.utility_mut().data;
        data[COSTS].copy_from_slice(&offers.costs);
        data[CLUES].copy_from_slice(&offers.clues);
        data[LEVEL_CLUES].copy_from_slice(&offers.level_clues);
    }

    /// The first slot's item when `ItemStack.isEnchantable` (`ItemStack.java:986-993`).
    pub fn enchantable_input(&self) -> Option<RuntimeItemStack> {
        if !self.is_enchantment_menu() {
            return None;
        }
        let item = &self.container[0];
        let enchantable = !item.is_empty()
            && has_component(item.summary(), VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID)
                .unwrap_or(false)
            && has_component(item.summary(), VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID)
                .unwrap_or(false)
            && enchantment_list(item, VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID).is_empty();
        enchantable.then(|| item.clone())
    }

    /// The enchanted item and the lapis of `clickMenuButton`
    /// (`EnchantmentMenu.java:150-166`): a book becomes an enchanted book.
    pub fn apply_enchantments(
        &mut self,
        enchantments: &[(usize, i32)],
        lapis: i32,
        infinite_materials: bool,
    ) -> Result<RuntimeItemStack> {
        let Some(facts) = self.utility().facts.clone() else {
            return Ok(RuntimeItemStack::empty());
        };
        let mut item = self.container[0].clone();
        if item.item_id() == Some(VANILLA_ITEM_BOOK_ID) {
            item = transmute(&item, VANILLA_ITEM_ENCHANTED_BOOK_ID, &facts.resolver)?;
        }
        let mut current = crafting_enchantments(&item);
        for &(holder, level) in enchantments {
            upgrade(&mut current, holder, level);
        }
        let item = with_crafting_enchantments(&item, &current, &facts.resolver)?;
        self.container[0] = item.clone();
        if !infinite_materials {
            self.container[1].shrink(lapis);
        }
        self.utility_mut().enchantment_inputs_changed = true;
        Ok(item)
    }

    /// `EnchantmentMenu.quickMoveStack` (`EnchantmentMenu.java:223-265`).
    pub(super) fn enchantment_quick_move_stack(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        slot: i16,
    ) -> Result<RuntimeItemStack> {
        let clicked = self.slot_item(slot).into_owned();
        let mut stack = clicked.clone();
        let moved = if slot == 0 || slot == 1 {
            self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 2, 38, true)?
        } else if stack.item_id() == Some(VANILLA_ITEM_LAPIS_LAZULI_ID) {
            self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 1, 2, true)?
        } else {
            if !self.container[0].is_empty() {
                return Ok(RuntimeItemStack::empty());
            }
            let single = stack.split(1);
            self.set_by_player(resolver, maps.as_deref_mut(), 0, single)?;
            true
        };
        if !moved {
            return Ok(RuntimeItemStack::empty());
        }
        self.finish_utility_move(resolver, maps, slot, &clicked, stack)
    }
}
