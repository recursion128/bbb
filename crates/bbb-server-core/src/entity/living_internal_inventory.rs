//! Living InventoryCarrier slots and AbstractHorse's separate mount cargo slots.

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ALLAY_ID, VANILLA_ENTITY_TYPE_PIGLIN_ID, VANILLA_ENTITY_TYPE_PILLAGER_ID,
    VANILLA_ENTITY_TYPE_VILLAGER_ID, VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
};
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::entity::general_living::is_abstract_horse_entity_type;
use crate::entity::horse_phenotype::HorsePhenotypeState;
use crate::entity::GeneralLivingData;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LivingInternalInventory {
    items: Vec<Option<RuntimeItemStack>>,
}

pub(crate) const fn plain_inventory_capacity(entity_type_id: i32) -> Option<usize> {
    match entity_type_id {
        VANILLA_ENTITY_TYPE_ALLAY_ID => Some(1),
        VANILLA_ENTITY_TYPE_VILLAGER_ID
        | VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID
        | VANILLA_ENTITY_TYPE_PIGLIN_ID => Some(8),
        VANILLA_ENTITY_TYPE_PILLAGER_ID => Some(5),
        _ => None,
    }
}

impl LivingInternalInventory {
    pub(crate) fn new(entity_type_id: i32) -> Option<Self> {
        let size = if let Some(size) = plain_inventory_capacity(entity_type_id) {
            size
        } else if is_abstract_horse_entity_type(entity_type_id) {
            0
        } else {
            return None;
        };
        Some(Self {
            items: vec![None; size],
        })
    }

    /// `SimpleContainer.addItem` (`SimpleContainer.java:71-84`).
    pub(crate) fn add_item(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut stack: RuntimeItemStack,
    ) -> RuntimeItemStack {
        let max = stack.max_stack_size().unwrap_or(64);
        for slot in &mut self.items {
            if slot.as_ref().is_some_and(RuntimeItemStack::is_empty) {
                *slot = None;
            }
            let Some(slot) = slot else {
                continue;
            };
            if stack.is_empty() {
                break;
            }
            if slot.count() < max
                && slot
                    .same_item_same_effective_components(&stack, resolver)
                    .unwrap_or(false)
            {
                let moved = (max - slot.count()).min(stack.count());
                slot.grow(moved);
                stack.shrink(moved);
            }
        }
        for slot in self.items.iter_mut().filter(|slot| slot.is_none()) {
            if stack.is_empty() {
                break;
            }
            *slot = Some(stack.split(stack.count()));
        }
        stack
    }

    /// `SimpleContainer.canAddItem` (`SimpleContainer.java:86-97`).
    pub(crate) fn can_add_item(
        &self,
        resolver: &dyn PersistentItemStackHolderResolver,
        stack: &RuntimeItemStack,
    ) -> bbb_server_types::Result<bool> {
        for slot in &self.items {
            let Some(target) = slot.as_ref().filter(|target| !target.is_empty()) else {
                return Ok(true);
            };
            if target.count() < target.max_stack_size().unwrap_or(64)
                && target.same_item_same_effective_components(stack, resolver)?
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(crate) fn item(
        &self,
        slot: usize,
    ) -> Option<&bbb_server_types::item_stack::RuntimeItemStack> {
        self.items
            .get(slot)?
            .as_ref()
            .filter(|stack| !stack.is_empty())
    }

    pub(crate) fn slots(&self) -> &[Option<RuntimeItemStack>] {
        &self.items
    }

    pub(crate) fn original_items(&self) -> impl Iterator<Item = &RuntimeItemStack> {
        self.items.iter().flatten()
    }

    pub(crate) fn update_count(&mut self, slot: usize, count: i32) {
        if let Some(original) = self.items.get_mut(slot).and_then(Option::as_mut) {
            original.set_count(count);
        }
    }

    pub(crate) fn set_item(&mut self, slot: usize, stack: Option<RuntimeItemStack>) -> bool {
        let Some(target) = self.items.get_mut(slot) else {
            return false;
        };
        *target = stack.filter(|stack| !stack.is_empty());
        true
    }

    pub(crate) fn remove_all_items(&mut self) -> Vec<(usize, RuntimeItemStack)> {
        self.items
            .iter_mut()
            .enumerate()
            .filter_map(|(slot, stack)| {
                stack
                    .take()
                    .filter(|stack| !stack.is_empty())
                    .map(|stack| (slot, stack))
            })
            .collect()
    }

    pub(crate) fn resize_for_horse(&mut self, state: HorsePhenotypeState) {
        self.items.resize(state.inventory_size(), None);
        for slot in &mut self.items {
            if slot.as_ref().is_some_and(RuntimeItemStack::is_empty) {
                *slot = None;
            }
        }
    }
}

impl GeneralLivingData {
    pub(crate) fn internal_inventory(&self) -> Option<&LivingInternalInventory> {
        self.internal_inventory.as_ref()
    }

    pub(crate) fn internal_inventory_mut(&mut self) -> Option<&mut LivingInternalInventory> {
        self.internal_inventory.as_mut()
    }
}

#[cfg(test)]
#[path = "living_internal_inventory/tests.rs"]
mod tests;
