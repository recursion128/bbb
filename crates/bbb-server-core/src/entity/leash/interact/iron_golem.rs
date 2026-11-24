use bbb_protocol::item_types::VANILLA_ITEM_IRON_INGOT_ID;
use bbb_protocol::packets::SoundSource;

use super::LeashInteractingPlayer;
use crate::entity::animal_feeding::{use_player_item, FeedingPlayer};
use crate::entity::store::{EntityStore, PendingEntitySound};
use crate::entity::EntityData;

/// `IronGolem.mobInteract` (`IronGolem.java:259-275`).
pub(super) fn mob_interact(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut LeashInteractingPlayer<'_>,
) -> bool {
    if player.hand_stack.item_id != Some(VANILLA_ITEM_IRON_INGOT_ID) {
        return false;
    }
    let Some(target) = store.get_mut(target_id) else {
        return false;
    };
    if !matches!(target.data(), EntityData::GeneralLiving(data)
        if data.entity_resource_id() == "minecraft:iron_golem")
    {
        return false;
    }
    let Some(health_before) = target.living().map(|living| living.health()) else {
        return false;
    };
    crate::entity::living::set_health(target, health_before + 25.0);
    if target
        .living()
        .is_none_or(|living| living.health() == health_before)
    {
        return false;
    }
    let position = target.position();
    let silent = target.is_silent();
    // `Entity.playSound` guards Silent and uses NEUTRAL (`Entity.java:1404-1407`, `:3548-3550`).
    if !silent {
        let random = store.random();
        let pitch = 1.0 + (random.next_float() - random.next_float()) * 0.2;
        store.play_entity_sound(PendingEntitySound {
            position,
            resource_id: "minecraft:entity.iron_golem.repair".to_owned(),
            source: SoundSource::Neutral,
            volume: 1.0,
            pitch,
        });
    }
    let _ = use_player_item(
        &mut FeedingPlayer {
            hand_stack: &mut *player.hand_stack,
            uuid: player.uuid,
            infinite_materials: player.infinite_materials,
        },
        false,
    );
    true
}
