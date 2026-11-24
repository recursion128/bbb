//! Live living-entity item-use flags and vibrations.

use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::InteractionHand;

use crate::entity::Entity;
use crate::world::simulation_level::LevelCtx;

pub(crate) fn set_using_item(
    entity: &mut Entity,
    level: &mut LevelCtx<'_, '_>,
    hand: Option<InteractionHand>,
) {
    let Some(living) = entity.living() else {
        return;
    };
    let flags = living.living_entity_flags();
    let was_using = flags & 1 != 0;
    if hand.is_some() == was_using {
        return;
    }
    let offhand = hand.map_or(flags & 2 != 0, |hand| hand == InteractionHand::OffHand);
    let slot = if offhand {
        EquipmentSlot::OffHand
    } else {
        EquipmentSlot::MainHand
    };
    let item = living
        .equipment()
        .get(slot)
        .filter(|stack| !stack.is_empty());
    if hand.is_some() && item.is_none() {
        return;
    }
    let vibrates = item.is_some_and(|item| {
        bbb_server_types::item_stack::rules::interact_vibrations(item.summary())
    });
    let living = entity.living_mut().unwrap();
    living.set_living_entity_flag(1, hand.is_some());
    if hand.is_some() {
        living.set_living_entity_flag(2, offhand);
    }
    entity
        .mark_synched_dirty(bbb_protocol::entity_synched_data::VANILLA_LIVING_ENTITY_FLAGS_DATA_ID);
    if vibrates {
        let event = if hand.is_some() {
            crate::level_event::VanillaGameEvent::ITEM_INTERACT_START
        } else {
            crate::level_event::VanillaGameEvent::ITEM_INTERACT_FINISH
        };
        level.game_event_at(event, entity.position(), Some(entity.id()));
    }
}
