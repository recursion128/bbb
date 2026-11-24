use bbb_protocol::equipment_slots::EquipmentSlot;

use super::{LeashInteractingPlayer, LeashInteraction};
use crate::entity::animal_feeding::FeedingPlayer;
use crate::entity::{EntityData, EntityRegistryTagView, EntityStore};

/// `HappyGhast.mobInteract` (`HappyGhast.java:272-291`).
pub(super) fn mob_interact(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut LeashInteractingPlayer<'_>,
    tags: &EntityRegistryTagView,
) -> Option<LeashInteraction> {
    let target = store.get(target_id)?;
    if !matches!(target.data(), EntityData::GeneralLiving(data)
        if data.happy_ghast_state().is_some() && !data.is_baby())
    {
        return None;
    }
    if player.hand_stack.count > 0 && player.hand_stack.item_id.is_some() {
        let creative_hand = player.infinite_materials.then(|| player.hand_stack.clone());
        match crate::entity::item_slots::equip_on_interact(
            store,
            target_id,
            &mut FeedingPlayer {
                hand_stack: &mut *player.hand_stack,
                uuid: player.uuid,
                infinite_materials: player.infinite_materials,
            },
            tags,
        ) {
            Ok(Some(effects)) => {
                // Player.java:835-838: creative equipping retains the original hand.
                if let Some(hand) = creative_hand {
                    *player.hand_stack = hand;
                }
                return Some(LeashInteraction::MountEquipped {
                    entity_id: target_id,
                    effects,
                });
            }
            Ok(None) => {}
            Err(()) => return Some(LeashInteraction::Refused),
        }
    }
    if player.secondary_use_active {
        return None;
    }
    match crate::entity::living::valid_equipment::has_valid_item_with_registry_tags(
        store.get(target_id)?,
        tags,
        EquipmentSlot::Body,
    ) {
        Some(true) => Some(LeashInteraction::Mounted {
            vehicle_entity_id: target_id,
            rotation: None,
        }),
        Some(false) => None,
        None => Some(LeashInteraction::Refused),
    }
}
