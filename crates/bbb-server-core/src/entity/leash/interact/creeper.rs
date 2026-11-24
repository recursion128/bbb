use bbb_protocol::item_types::{vanilla_item_type, VANILLA_ITEM_FIRE_CHARGE_ID};
use bbb_protocol::packets::{ItemStackSummary, SoundSource};

use super::LeashInteractingPlayer;
use crate::entity::level::EntityRegistryTagView;
use crate::entity::store::{EntityStore, PendingEntitySound};
use crate::entity::EntityData;

/// `Creeper.mobInteract` (`Creeper.java:210-227`).
pub(super) fn mob_interact(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut LeashInteractingPlayer<'_>,
    tags: &EntityRegistryTagView,
) -> Option<bool> {
    let item = player.hand_stack.item_id.and_then(vanilla_item_type)?;
    if !tags.contains(
        "minecraft:item",
        "minecraft:creeper_igniters",
        item.resource_id,
    ) {
        return None;
    }
    let target = store.get_mut(target_id)?;
    if !target.is_alive_with_health()
        || !matches!(target.data(), EntityData::GeneralLiving(data)
        if data.creeper_retained().is_some())
    {
        return None;
    }
    let position = target.position();
    crate::entity::ai::mob::creeper::ignite(target);
    // Creeper.java:214: level.playSound(player, ...) excludes the igniting player.
    let pitch = 0.8 + store.random().next_float() * 0.4;
    store.play_entity_sound_except(
        player.entity_id,
        PendingEntitySound {
            position,
            resource_id: if player.hand_stack.item_id == Some(VANILLA_ITEM_FIRE_CHARGE_ID) {
                "minecraft:item.firecharge.use"
            } else {
                "minecraft:item.flintandsteel.use"
            }
            .to_owned(),
            source: SoundSource::Hostile,
            volume: 1.0,
            pitch,
        },
    );
    let damage_tool =
        bbb_server_types::item_stack::rules::damageable_max_damage(player.hand_stack).is_some();
    if !damage_tool {
        player.hand_stack.count -= 1;
        if player.hand_stack.count <= 0 {
            *player.hand_stack = ItemStackSummary::empty();
        }
    }
    Some(damage_tool)
}
