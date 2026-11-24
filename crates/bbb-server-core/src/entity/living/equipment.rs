use std::collections::BTreeMap;

use bbb_protocol::equipment_slots::{EquipmentSlot, VANILLA_EQUIPMENT_SLOTS};
use bbb_server_types::item_stack::RuntimeItemStack;

// EntityEquipment.java:10-70: one slot owner.
#[derive(Debug, Clone, Default)]
pub struct EntityEquipment {
    items: BTreeMap<EquipmentSlot, RuntimeItemStack>,
}

impl PartialEq for EntityEquipment {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

impl EntityEquipment {
    /// `new EntityEquipment()` (`EntityEquipment.java:26-28`).
    pub fn new() -> Self {
        Self::default()
    }

    /// `set(EquipmentSlot, ItemStack)` (`EntityEquipment.java:30-32`): the
    /// previous stack, `None` standing for `ItemStack.EMPTY`.
    pub fn set(
        &mut self,
        slot: EquipmentSlot,
        stack: Option<RuntimeItemStack>,
    ) -> Option<RuntimeItemStack> {
        if let (Some(current), Some(next)) = (self.items.get(&slot), stack.as_ref()) {
            if current == next {
                return stack;
            }
        }
        let previous = self.items.remove(&slot);
        if let Some(stack) = stack.filter(|stack| !stack.is_empty()) {
            self.items.insert(slot, stack);
        }
        previous
    }

    /// `get(EquipmentSlot)` (`EntityEquipment.java:34-36`), `None` for
    /// `ItemStack.EMPTY`.
    pub fn get(
        &self,
        slot: EquipmentSlot,
    ) -> Option<&bbb_server_types::item_stack::RuntimeItemStack> {
        self.items.get(&slot).filter(|stack| !stack.is_empty())
    }

    /// `isEmpty()` (`EntityEquipment.java:38-46`).
    pub fn is_empty(&self) -> bool {
        self.iter().next().is_none()
    }

    /// `setAll(EntityEquipment)` (`EntityEquipment.java:57-60`).
    pub fn set_all(&mut self, other: &Self) {
        self.items.clone_from(&other.items);
    }

    /// `clear()` (`EntityEquipment.java:70-72`).
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Every non-empty slot in `EquipmentSlot.VALUES` order, which is what
    /// the codec writes and what `EquipmentSlot.VALUES` walks
    /// (`EquipmentSlot.java:23`).
    pub fn iter(
        &self,
    ) -> impl Iterator<
        Item = (
            EquipmentSlot,
            &bbb_server_types::item_stack::RuntimeItemStack,
        ),
    > {
        VANILLA_EQUIPMENT_SLOTS
            .iter()
            .filter_map(|entry| self.get(entry.slot).map(|stack| (entry.slot, stack)))
    }

    pub(crate) fn referenced_item(
        &self,
        slot: EquipmentSlot,
    ) -> Option<&bbb_server_types::item_stack::RuntimeItemStack> {
        self.items.get(&slot)
    }

    pub(crate) fn update_item(
        &mut self,
        slot: EquipmentSlot,
        stack: RuntimeItemStack,
    ) -> Option<()> {
        *self.items.get_mut(&slot)? = stack;
        Some(())
    }
}
