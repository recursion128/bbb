//! `BeaconMenu`'s payment slot; the effects live on the block entity.

use crate::world::maps::ServerMapStore;
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result;

use super::PlayerInventoryMenu;

impl PlayerInventoryMenu {
    /// `BeaconMenu.PaymentSlot.mayPlace` (`BeaconMenu.java:161-163`).
    pub(super) fn beacon_payment_item(&self, item: &RuntimeItemStack) -> bool {
        self.item_tag_contains("minecraft:beacon_payment_items", item)
    }

    /// `BeaconMenu.updateEffects`'s payment half (`BeaconMenu.java:142-149`):
    /// whether there was a payment, which is then spent.
    pub fn spend_beacon_payment(&mut self) -> bool {
        if self.utility_menu_kind() != Some(super::UtilityMenuKind::Beacon)
            || self.container[0].is_empty()
        {
            return false;
        }
        self.container[0].shrink(1);
        true
    }

    /// `BeaconMenu.quickMoveStack` (`BeaconMenu.java:78-117`).
    pub(super) fn beacon_quick_move_stack(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        slot: i16,
    ) -> Result<RuntimeItemStack> {
        let clicked = self.slot_item(slot).into_owned();
        let mut stack = clicked.clone();
        let moved = if slot == 0 {
            self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 1, 37, true)?
        } else if self.container[0].is_empty()
            && self.beacon_payment_item(&stack)
            && stack.count() == 1
        {
            self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 0, 1, false)?
        } else if (1..28).contains(&slot) {
            self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 28, 37, false)?
        } else if (28..37).contains(&slot) {
            self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 1, 28, false)?
        } else {
            self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 1, 37, false)?
        };
        if !moved {
            return Ok(RuntimeItemStack::empty());
        }
        self.finish_utility_move(resolver, maps, slot, &clicked, stack)
    }
}
