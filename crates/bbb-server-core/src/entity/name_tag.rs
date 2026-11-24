//! `NameTagItem.interactLivingEntity` (`NameTagItem.java:17-33`).

use bbb_protocol::entity_types::vanilla_entity_type_is_serializable;
use bbb_protocol::packets::ItemStackSummary;
use bbb_protocol::RichTextComponent;

use crate::entity::store::EntityStore;
use crate::entity::{Entity, EntityData};

/// What `NameTagItem.interactLivingEntity` answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NameTagInteraction {
    /// `InteractionResult.PASS` (`NameTagItem.java:31`).
    Pass,
    /// `SUCCESS` (`:29`): the caller owes `ENTITY_INTERACT` and a client swing.
    Success,
}

/// `LivingEntity.isAlive` (`LivingEntity.java:1718-1720`).
pub(crate) fn living_entity_is_alive(entity: &Entity) -> bool {
    entity.is_alive() && entity.living().is_some_and(|living| living.health() > 0.0)
}

/// `itemStack.get(DataComponents.CUSTOM_NAME)` (`NameTagItem.java:18`).
pub(crate) fn custom_name_from_stack(stack: &ItemStackSummary) -> Option<RichTextComponent> {
    let patch = &stack.component_patch;
    if let Some(nbt) = patch.custom_name_nbt.clone() {
        if let Ok(component) = RichTextComponent::try_from_component_nbt(nbt) {
            return Some(component);
        }
    }
    if let Some(text) = patch.custom_name.clone() {
        return Some(RichTextComponent::plain(text));
    }
    patch
        .custom_name_styled
        .as_ref()
        .and_then(|runs| runs.first())
        .map(|run| RichTextComponent::plain(run.text.clone()))
}

/// `NameTagItem.interactLivingEntity` (`NameTagItem.java:17-33`).
///
/// Any living target takes this path, which `Player.interactOn` reaches for
/// a target whose own `interact` passed (`Player.java:841-850`).
pub(crate) fn interact_living_entity(
    store: &mut EntityStore,
    target_id: i32,
    hand_stack: &mut ItemStackSummary,
) -> NameTagInteraction {
    let Some(custom_name) = custom_name_from_stack(hand_stack) else {
        return NameTagInteraction::Pass;
    };
    let Some(target) = store.get(target_id) else {
        return NameTagInteraction::Pass;
    };
    if target.living().is_none() {
        return NameTagInteraction::Pass;
    }
    let entity_type_id = target.entity_type_id();
    // `:19`.
    let serializable = vanilla_entity_type_is_serializable(entity_type_id)
        || store
            .plugins()
            .content_registry()
            .entity_by_id(entity_type_id)
            .is_some_and(|spec| spec.serializable);
    if !serializable {
        return NameTagInteraction::Pass;
    }
    let Some(target) = store.get_mut(target_id) else {
        return NameTagInteraction::Pass;
    };
    // `:20-26`: this crate is the server; a dead living skips the apply.
    if living_entity_is_alive(target) {
        target.set_custom_name(Some(custom_name));
        if let EntityData::GeneralLiving(data) = &mut target.data {
            if let Some(mob) = data.mob.as_mut() {
                // `Mob.setPersistenceRequired` (`Mob.java:1068-1070`).
                mob.set_persistence_required(true);
            }
        }
        // `:26`.
        hand_stack.count -= 1;
        if hand_stack.count <= 0 {
            *hand_stack = ItemStackSummary::empty();
        }
    }
    NameTagInteraction::Success
}
