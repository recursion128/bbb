//! `MerchantMenu` over its own `MerchantContainer`, reading a working copy of
//! the trader's offers the world refreshes before every menu operation.

use crate::world::maps::ServerMapStore;
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_types::Result;

use super::{MenuLayout, PlayerInventoryMenu};
use bbb_server_types::item_stack::RuntimeItemStack;

/// `MerchantMenu.PAYMENT1_SLOT` .. `RESULT_SLOT` (`MerchantMenu.java:17-19`).
const PAYMENT_A: usize = 0;
const PAYMENT_B: usize = 1;
const RESULT: usize = 2;
const RESULT_SLOT: i16 = 2;
/// `MerchantMenu.INV_SLOT_START` .. `USE_ROW_SLOT_END` (`MerchantMenu.java:20-23`).
const INV_SLOT_START: i16 = 3;
const INV_SLOT_END: i16 = 30;
const USE_ROW_SLOT_END: i16 = 39;

/// One `ItemCost` as the menu tests it: `ItemCost.test` (`ItemCost.java:51-53`)
/// and the count `MerchantOffer.satisfiedBy` compares (`MerchantOffer.java:213-219`).
#[derive(Debug, Clone, PartialEq)]
pub struct MerchantTradeCost {
    pub item_id: i32,
    /// `getModifiedCostCount` for the first cost, the plain count for the second.
    pub count: i32,
    /// The cost's `DataComponentExactPredicate`, absent when it names none.
    pub exact_components: Option<bbb_protocol::packets::DataComponentPatchSummary>,
}

/// The fields of one `MerchantOffer` the menu reads.
#[derive(Debug, Clone, PartialEq)]
pub struct MerchantTrade {
    pub cost_a: MerchantTradeCost,
    pub cost_b: Option<MerchantTradeCost>,
    pub result: RuntimeItemStack,
    pub uses: i32,
    pub max_uses: i32,
    pub xp: i32,
}

/// What a menu operation owes the trader and the player, in the order the
/// menu reached it.
#[derive(Debug, Clone, PartialEq)]
pub enum MerchantMenuEvent {
    /// `MerchantResultSlot.checkTakeAchievements` (`MerchantResultSlot.java:43-46`).
    Crafted { item_id: i32, count: i32 },
    /// A successful `MerchantOffer.take` in `MerchantResultSlot.onTake`
    /// (`MerchantResultSlot.java:55-60`): `notifyTrade` and the statistic.
    Traded { offer: usize },
    /// `Merchant.notifyTradeUpdated` from `MerchantContainer.updateSellItem`
    /// (`MerchantContainer.java:122`).
    TradeUpdated { valid: bool },
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(super) struct MerchantMenuState {
    trader: i32,
    trades: Vec<MerchantTrade>,
    active_offer: Option<usize>,
    selection_hint: i32,
    future_xp: i32,
    remove_count: i32,
    events: Vec<MerchantMenuEvent>,
}

impl PlayerInventoryMenu {
    pub fn is_merchant_menu(&self) -> bool {
        self.menu.layout == MenuLayout::Merchant
    }

    /// The entity id of the trader behind the open merchant menu.
    pub fn merchant_trader(&self) -> Option<i32> {
        self.menu.merchant.as_ref().map(|merchant| merchant.trader)
    }

    pub fn bind_merchant_trader(&mut self, trader: i32) {
        if let Some(merchant) = self.menu.merchant.as_mut() {
            merchant.trader = trader;
        }
    }

    /// Replaces the working copy of `Merchant.getOffers()`.
    pub fn refresh_merchant_trades(&mut self, trades: Vec<MerchantTrade>) {
        let Some(merchant) = self.menu.merchant.as_mut() else {
            return;
        };
        if merchant.trades.len() != trades.len() {
            merchant.active_offer = None;
        }
        merchant.trades = trades;
    }

    pub fn merchant_trades(&self) -> &[MerchantTrade] {
        self.menu
            .merchant
            .as_ref()
            .map_or(&[], |merchant| merchant.trades.as_slice())
    }

    /// `MerchantContainer.getFutureXp` (`MerchantContainer.java:140-142`).
    pub fn merchant_future_xp(&self) -> i32 {
        self.menu
            .merchant
            .as_ref()
            .map_or(0, |merchant| merchant.future_xp)
    }

    pub fn take_merchant_events(&mut self) -> Vec<MerchantMenuEvent> {
        self.menu
            .merchant
            .as_mut()
            .map(|merchant| std::mem::take(&mut merchant.events))
            .unwrap_or_default()
    }

    /// `ServerGamePacketListenerImpl.handleSelectTrade` (`:971-972`):
    /// `setSelectionHint` then `tryMoveItems`.
    pub(crate) fn select_merchant_trade(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        maps: Option<&mut ServerMapStore>,
        selection: i32,
    ) -> Result<()> {
        if !self.is_merchant_menu() {
            return Ok(());
        }
        self.menu
            .merchant
            .as_mut()
            .expect("merchant layout")
            .selection_hint = selection;
        self.update_sell_item(resolver)?;
        self.try_move_merchant_items(resolver, maps, selection)
    }

    /// `MerchantMenu.tryMoveItems` (`MerchantMenu.java:171-197`).
    fn try_move_merchant_items(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        selection: i32,
    ) -> Result<()> {
        let Ok(index) = usize::try_from(selection) else {
            return Ok(());
        };
        let Some(trade) = self.merchant_trades().get(index).cloned() else {
            return Ok(());
        };
        for payment in [PAYMENT_A, PAYMENT_B] {
            let mut old = self.container[payment].clone();
            if old.is_empty() {
                continue;
            }
            if !self.move_item_stack_to(
                resolver,
                maps.as_deref_mut(),
                &mut old,
                INV_SLOT_START,
                USE_ROW_SLOT_END,
                true,
            )? {
                return Ok(());
            }
            self.container[payment] = old;
            self.update_sell_item(resolver)?;
        }
        if self.container[PAYMENT_A].is_empty() && self.container[PAYMENT_B].is_empty() {
            self.move_from_inventory_to_payment_slot(
                resolver,
                maps.as_deref_mut(),
                PAYMENT_A,
                &trade.cost_a,
            )?;
            if let Some(cost_b) = &trade.cost_b {
                self.move_from_inventory_to_payment_slot(resolver, maps, PAYMENT_B, cost_b)?;
            }
        }
        Ok(())
    }

    /// `MerchantMenu.moveFromInventoryToPaymentSlot` (`MerchantMenu.java:199-216`).
    fn move_from_inventory_to_payment_slot(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        payment: usize,
        cost: &MerchantTradeCost,
    ) -> Result<()> {
        for slot in INV_SLOT_START..USE_ROW_SLOT_END {
            let inventory_item = self.slot_item(slot).into_owned();
            if inventory_item.is_empty() || !cost_test(resolver, cost, &inventory_item)? {
                continue;
            }
            let current = self.container[payment].clone();
            if !current.is_empty()
                && !inventory_item.same_item_same_effective_components(&current, resolver)?
            {
                continue;
            }
            let max = inventory_item.max_stack_size()?;
            let current_count = if current.is_empty() {
                0
            } else {
                current.count()
            };
            let moved = (max - current_count).min(inventory_item.count());
            let payment_item = inventory_item.with_count(current_count + moved);
            self.set_slot_item_in_place(
                resolver,
                maps.as_deref_mut(),
                slot,
                inventory_item.with_count(inventory_item.count() - moved),
            )?;
            let full = payment_item.count() >= max;
            self.set_merchant_slot(resolver, payment, payment_item)?;
            if full {
                break;
            }
        }
        Ok(())
    }

    /// `MerchantContainer.setItem` (`MerchantContainer.java:70-76`).
    fn set_merchant_slot(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        slot: usize,
        item: RuntimeItemStack,
    ) -> Result<()> {
        self.container[slot] = item;
        if slot == PAYMENT_A || slot == PAYMENT_B {
            self.update_sell_item(resolver)?;
        }
        Ok(())
    }

    /// `MerchantContainer.updateSellItem` (`MerchantContainer.java:88-124`).
    pub(super) fn update_sell_item(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> Result<()> {
        if !self.is_merchant_menu() {
            return Ok(());
        }
        let (buy_a, buy_b) = if self.container[PAYMENT_A].is_empty() {
            (self.container[PAYMENT_B].clone(), RuntimeItemStack::empty())
        } else {
            (
                self.container[PAYMENT_A].clone(),
                self.container[PAYMENT_B].clone(),
            )
        };
        let merchant = self.menu.merchant.as_mut().expect("merchant layout");
        merchant.active_offer = None;
        if buy_a.is_empty() {
            self.container[RESULT] = RuntimeItemStack::empty();
            merchant.future_xp = 0;
            return Ok(());
        }
        if !merchant.trades.is_empty() {
            let hint = merchant.selection_hint;
            let mut offer = recipe_for(resolver, &merchant.trades, &buy_a, &buy_b, hint)?;
            if offer.is_none_or(|index| out_of_stock(&merchant.trades[index])) {
                merchant.active_offer = offer;
                offer = recipe_for(resolver, &merchant.trades, &buy_b, &buy_a, hint)?;
            }
            match offer.filter(|index| !out_of_stock(&merchant.trades[*index])) {
                Some(index) => {
                    merchant.active_offer = Some(index);
                    self.container[RESULT] = merchant.trades[index].result.clone();
                    merchant.future_xp = merchant.trades[index].xp;
                }
                None => {
                    self.container[RESULT] = RuntimeItemStack::empty();
                    merchant.future_xp = 0;
                }
            }
        }
        let valid = !self.container[RESULT].is_empty();
        merchant
            .events
            .push(MerchantMenuEvent::TradeUpdated { valid });
        Ok(())
    }

    /// `MerchantResultSlot.remove` (`MerchantResultSlot.java:28-34`) over
    /// `MerchantContainer.removeItem` (`MerchantContainer.java:46-58`), which
    /// hands over the whole result whatever was asked for.
    pub(super) fn merchant_result_remove(&mut self, amount: i32) -> RuntimeItemStack {
        let count = self.container[RESULT].count();
        let taken = self.container[RESULT].split(count);
        if let Some(merchant) = self.menu.merchant.as_mut() {
            merchant.remove_count += amount.min(taken.count());
        }
        taken
    }

    /// `MerchantResultSlot.checkTakeAchievements` (`MerchantResultSlot.java:43-46`).
    fn merchant_check_take_achievements(&mut self, carried: &RuntimeItemStack) {
        let merchant = self.menu.merchant.as_mut().expect("merchant layout");
        let count = std::mem::take(&mut merchant.remove_count);
        if let Some(item_id) = carried.item_id().filter(|_| count != 0) {
            merchant
                .events
                .push(MerchantMenuEvent::Crafted { item_id, count });
        }
    }

    /// `MerchantResultSlot.onTake` (`MerchantResultSlot.java:49-64`).
    pub(super) fn on_take_merchant_result(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        carried: &RuntimeItemStack,
    ) -> Result<()> {
        self.merchant_check_take_achievements(carried);
        let merchant = self.menu.merchant.as_mut().expect("merchant layout");
        let Some(index) = merchant.active_offer else {
            return Ok(());
        };
        let trade = merchant.trades[index].clone();
        let mut buy_a = self.container[PAYMENT_A].clone();
        let mut buy_b = self.container[PAYMENT_B].clone();
        if take(resolver, &trade, &mut buy_a, &mut buy_b)?
            || take(resolver, &trade, &mut buy_b, &mut buy_a)?
        {
            // `AbstractVillager.notifyTrade` (`AbstractVillager.java:134-141`)
            // increases the uses the next `updateSellItem` reads.
            merchant.trades[index].uses = merchant.trades[index].uses.wrapping_add(1);
            merchant
                .events
                .push(MerchantMenuEvent::Traded { offer: index });
            self.container[PAYMENT_A] = buy_a;
            self.container[PAYMENT_B] = buy_b;
            self.update_sell_item(resolver)?;
            self.update_sell_item(resolver)?;
        }
        Ok(())
    }

    /// `MerchantResultSlot.onQuickCraft` (`MerchantResultSlot.java:37-40`).
    fn merchant_on_quick_craft(&mut self, original: &RuntimeItemStack, count: i32) {
        if let Some(merchant) = self.menu.merchant.as_mut() {
            merchant.remove_count += count;
        }
        self.merchant_check_take_achievements(original);
    }

    /// `MerchantMenu.quickMoveStack` (`MerchantMenu.java:101-140`).
    pub(super) fn merchant_quick_move_stack(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        slot: i16,
        dropped: &mut Vec<super::DroppedMenuStack>,
    ) -> Result<RuntimeItemStack> {
        if !(0..USE_ROW_SLOT_END).contains(&slot) {
            return Ok(RuntimeItemStack::empty());
        }
        let clicked = self.slot_item(slot).into_owned();
        if clicked.is_empty() {
            return Ok(RuntimeItemStack::empty());
        }
        let mut stack = clicked.clone();
        if slot == RESULT_SLOT {
            if !self.move_item_stack_to(
                resolver,
                maps.as_deref_mut(),
                &mut stack,
                INV_SLOT_START,
                USE_ROW_SLOT_END,
                true,
            )? {
                return Ok(RuntimeItemStack::empty());
            }
            let moved = clicked.count() - if stack.is_empty() { 0 } else { stack.count() };
            if moved > 0 {
                self.merchant_on_quick_craft(&clicked, moved);
            }
        } else if slot != 0 && slot != 1 {
            if (INV_SLOT_START..INV_SLOT_END).contains(&slot) {
                if !self.move_item_stack_to(
                    resolver,
                    maps.as_deref_mut(),
                    &mut stack,
                    INV_SLOT_END,
                    USE_ROW_SLOT_END,
                    false,
                )? {
                    return Ok(RuntimeItemStack::empty());
                }
            } else if (INV_SLOT_END..USE_ROW_SLOT_END).contains(&slot)
                && !self.move_item_stack_to(
                    resolver,
                    maps.as_deref_mut(),
                    &mut stack,
                    INV_SLOT_START,
                    INV_SLOT_END,
                    false,
                )?
            {
                return Ok(RuntimeItemStack::empty());
            }
        } else if !self.move_item_stack_to(
            resolver,
            maps.as_deref_mut(),
            &mut stack,
            INV_SLOT_START,
            USE_ROW_SLOT_END,
            false,
        )? {
            return Ok(RuntimeItemStack::empty());
        }
        let remaining = if stack.is_empty() { 0 } else { stack.count() };
        let remainder = if slot == RESULT_SLOT {
            stack.clone()
        } else {
            RuntimeItemStack::empty()
        };
        self.finish_slot_move(resolver, maps, slot, stack.clone())?;
        if remaining == clicked.count() {
            return Ok(RuntimeItemStack::empty());
        }
        if slot == RESULT_SLOT {
            self.on_take_merchant_result(resolver, &stack)?;
            if !remainder.is_empty() {
                dropped.push(super::DroppedMenuStack {
                    stack: remainder,
                    thrown_from_hand: false,
                });
            }
        } else if slot == 0 || slot == 1 {
            // `Slot.onTake` (`Slot.java:40-42`) ends in `setChanged`.
            self.update_sell_item(resolver)?;
        }
        Ok(clicked)
    }

    /// The two payment slots `MerchantMenu.removed` (`MerchantMenu.java:153-167`)
    /// hands back, emptied from the container.
    pub(super) fn take_merchant_payments(
        &mut self,
    ) -> [bbb_server_types::item_stack::RuntimeItemStack; 2] {
        [
            std::mem::replace(&mut self.container[PAYMENT_A], RuntimeItemStack::empty()),
            std::mem::replace(&mut self.container[PAYMENT_B], RuntimeItemStack::empty()),
        ]
    }
}

fn out_of_stock(trade: &MerchantTrade) -> bool {
    // `MerchantOffer.isOutOfStock` (`MerchantOffer.java:197-199`).
    trade.uses >= trade.max_uses
}

/// `MerchantOffers.getRecipeFor` (`MerchantOffers.java:34-48`).
fn recipe_for(
    resolver: &dyn PersistentItemStackHolderResolver,
    trades: &[MerchantTrade],
    buy_a: &RuntimeItemStack,
    buy_b: &RuntimeItemStack,
    selection_hint: i32,
) -> Result<Option<usize>> {
    if let Ok(hint) = usize::try_from(selection_hint) {
        if hint > 0 && hint < trades.len() {
            return Ok(satisfied_by(resolver, &trades[hint], buy_a, buy_b)?.then_some(hint));
        }
    }
    for (index, trade) in trades.iter().enumerate() {
        if satisfied_by(resolver, trade, buy_a, buy_b)? {
            return Ok(Some(index));
        }
    }
    Ok(None)
}

/// `MerchantOffer.satisfiedBy` (`MerchantOffer.java:213-219`).
fn satisfied_by(
    resolver: &dyn PersistentItemStackHolderResolver,
    trade: &MerchantTrade,
    buy_a: &RuntimeItemStack,
    buy_b: &RuntimeItemStack,
) -> Result<bool> {
    if !cost_test(resolver, &trade.cost_a, buy_a)? || buy_a.count() < trade.cost_a.count {
        return Ok(false);
    }
    match &trade.cost_b {
        None => Ok(buy_b.is_empty()),
        Some(cost_b) => Ok(cost_test(resolver, cost_b, buy_b)? && buy_b.count() >= cost_b.count),
    }
}

/// `MerchantOffer.take` (`MerchantOffer.java:221-232`).
fn take(
    resolver: &dyn PersistentItemStackHolderResolver,
    trade: &MerchantTrade,
    buy_a: &mut RuntimeItemStack,
    buy_b: &mut RuntimeItemStack,
) -> Result<bool> {
    if !satisfied_by(resolver, trade, buy_a, buy_b)? {
        return Ok(false);
    }
    buy_a.set_count(buy_a.count() - trade.cost_a.count);
    if let Some(cost_b) = &trade.cost_b {
        buy_b.set_count(buy_b.count() - cost_b.count);
    }
    Ok(true)
}

/// `ItemCost.test` (`ItemCost.java:51-53`).
fn cost_test(
    resolver: &dyn PersistentItemStackHolderResolver,
    cost: &MerchantTradeCost,
    stack: &RuntimeItemStack,
) -> Result<bool> {
    if stack.is_empty() || stack.item_id() != Some(cost.item_id) {
        return Ok(false);
    }
    let Some(predicate) = &cost.exact_components else {
        return Ok(true);
    };
    // DataComponentExactPredicate.test (`DataComponentExactPredicate.java:77-86`).
    Ok(predicate.added_type_ids.iter().all(|&type_id| {
        stack
            .effective_component(type_id, resolver)
            .ok()
            .flatten()
            .is_some_and(|actual| {
                bbb_protocol::packets::data_components::fields::component_values_equal(
                    type_id, predicate, &actual,
                )
                .unwrap_or(false)
            })
    }))
}
