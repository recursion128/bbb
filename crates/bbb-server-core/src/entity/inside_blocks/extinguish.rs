use crate::entity::store::{EntityStore, PendingEntitySound};
use crate::entity::{Entity, EntityData};

/// `Entity.playEntityOnFireExtinguishedSound` (`Entity.java:950-963`).
pub(crate) fn play_entity_extinguish_sound(entity: &Entity, store: &mut EntityStore) -> Option<()> {
    // `Fireball.java:43-44` overrides the sound, unlike DragonFireball.
    if matches!(
        entity.data(),
        EntityData::LargeFireball(_) | EntityData::SmallFireball(_)
    ) {
        return Some(());
    }
    let random = store.random();
    let pitch = 1.6_f32 + (random.next_float() - random.next_float()) * 0.4_f32;
    store.play_entity_sound(PendingEntitySound {
        position: entity.position(),
        resource_id: "minecraft:entity.generic.extinguish_fire".to_owned(),
        source: crate::entity::living::living_sound_source(entity),
        volume: 0.7,
        pitch,
    });
    Some(())
}
