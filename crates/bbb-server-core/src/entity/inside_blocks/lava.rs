use crate::entity::store::PendingEntitySound;
use crate::entity::{Entity, EntityData, EntityStore};
use crate::world::simulation_level::LevelCtx;

pub(crate) fn apply_lava_hurt<F>(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    amount: f32,
    hurt: &mut F,
) -> bool
where
    F: FnMut(
        &mut Entity,
        &mut EntityStore,
        &mut LevelCtx<'_, '_>,
        &'static str,
        f32,
    ) -> Option<bool>,
{
    match entity.fire_immune_in(level) {
        Ok(true) => return true,
        Err(_) => return false,
        Ok(false) => {}
    }
    let Some(succeeded) = hurt(entity, store, level, "minecraft:lava", amount) else {
        return false;
    };
    finish_lava_hurt(entity, store, succeeded)
}

pub(crate) fn finish_lava_hurt(entity: &Entity, store: &mut EntityStore, succeeded: bool) -> bool {
    if !succeeded {
        return true;
    }
    let should_play = match entity.data() {
        EntityData::Item(item) => item.health() <= 0 || entity.tick_count % 10 == 0,
        _ => true,
    };
    if !should_play || entity.is_silent() {
        return true;
    }
    let pitch = 2.0_f32 + store.random().next_float() * 0.4_f32;
    store.play_entity_sound(PendingEntitySound {
        position: entity.position(),
        resource_id: "minecraft:entity.generic.burn".to_owned(),
        source: crate::entity::living::living_sound_source(entity),
        volume: 0.4,
        pitch,
    });
    true
}

#[cfg(test)]
mod tests;
