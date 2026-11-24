//! `Ocelot.mobInteract`'s trust arm (`Ocelot.java:159-178`).

use bbb_protocol::entity_synched_data::VANILLA_OCELOT_TRUSTING_DATA_ID;
use bbb_protocol::packets::EntityEvent;

use super::{AnimalFeeding, FeedingPlayer, FoodPredicate};
use crate::connection::ClientboundPlayPacket;
use crate::entity::level::EntityRegistryTagView;
use crate::entity::{EntityData, EntityStore};

const OCELOT_FOOD: FoodPredicate = FoodPredicate::Tag("minecraft:ocelot_food");

/// `Ocelot.handleEntityEvent` hearts and smoke (`Ocelot.java:181-189`).
const TRUSTING_SUCCEEDED_ENTITY_EVENT: i8 = 41;
const TRUSTING_FAILED_ENTITY_EVENT: i8 = 40;

pub(super) fn mob_interact(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut FeedingPlayer<'_>,
    tags: &EntityRegistryTagView,
    mob_item_used: &mut bool,
) -> Option<AnimalFeeding> {
    let target = store.get(target_id)?;
    let EntityData::GeneralLiving(data) = target.data() else {
        return None;
    };
    let trusting = data.ocelot_trusting()?;
    let center = target.position();
    let near = store.player_by_uuid(player.uuid).is_none_or(|player| {
        let position = player.body().position();
        let (dx, dy, dz) = (
            position.x - center.x,
            position.y - center.y,
            position.z - center.z,
        );
        dx * dx + dy * dy + dz * dz < 9.0
    });
    let tempted = store
        .mob_ai(target_id)
        .is_some_and(crate::entity::ai::mob::MobAiRuntime::tempt_goal_running);
    if !tempted || trusting || !near || !OCELOT_FOOD.matches(player.hand_stack.item_id, tags) {
        return None;
    }
    super::use_mob_player_item(player, mob_item_used);
    let trusted = store.random().next_int(3) == 0;
    if trusted {
        let target = store.get_mut(target_id)?;
        if let EntityData::GeneralLiving(data) = &mut target.data {
            data.set_ocelot_trusting(true);
        }
        target.mark_synched_dirty(VANILLA_OCELOT_TRUSTING_DATA_ID);
    }
    store.broadcast_to_tracking(
        target_id,
        ClientboundPlayPacket::EntityEvent(EntityEvent {
            entity_id: target_id,
            event_id: if trusted {
                TRUSTING_SUCCEEDED_ENTITY_EVENT
            } else {
                TRUSTING_FAILED_ENTITY_EVENT
            },
        }),
    );
    Some(AnimalFeeding::Fed)
}
