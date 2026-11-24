//! Store-side state of the Mob pickup owner ([`crate::entity::mob_pickup`]): the
//! `PREVENT_ARMOR_CHANGE` enchantment set and the deferred
//! `THROWN_ITEM_PICKED_UP_BY_ENTITY` record.

use std::collections::BTreeSet;
use std::sync::Arc;

use bbb_server_types::item_stack::RuntimeItemStack;

use super::EntityStore;

/// One `LivingEntity.onItemPickup` (`LivingEntity.java:3246-3251`) whose item
/// entity named a thrower; the world preserves a cached session body through
/// the deferred criterion batch.
#[derive(Debug, Clone, PartialEq)]
pub struct PendingThrownItemPickup {
    pub thrower: crate::entity::entity_reference::EntityReference,
    /// The picking entity, still filed in the store when the record drains.
    pub entity_id: i32,
    /// `entity.getItem()` at the call, before the pickup shrinks it.
    pub item: RuntimeItemStack,
}

impl EntityStore {
    pub fn set_prevent_armor_change_enchantments(&mut self, enchantments: Arc<BTreeSet<String>>) {
        self.prevent_armor_change = enchantments;
    }

    pub(crate) fn prevents_armor_change(&self, enchantment_id: &str) -> bool {
        self.prevent_armor_change.contains(enchantment_id)
    }
}
