//! `Bat`'s AI effects, ambient sound and post-super motion.

use super::*;
use crate::world::simulation_level::LevelCtx;

pub(super) fn apply_bat_ai_step(
    entity: &mut Entity,
    level: &mut LevelCtx<'_, '_>,
    result: BatAiStepResult,
) {
    if let Some(resting) = result.resting {
        let changed = match &mut entity.data {
            EntityData::GeneralLiving(data) => data
                .bat_retained_mut()
                .is_some_and(|bat| bat.set_resting(resting)),
            _ => false,
        };
        if changed {
            entity.mark_synched_dirty(VANILLA_BAT_FLAGS_DATA_ID);
        }
    }
    if let Some(movement) = result.delta_movement {
        entity.set_delta_movement(movement);
    }
    if let Some(y_rot) = result.y_rot {
        entity.set_rot(y_rot, entity.x_rot());
    }
    if result.takeoff_event && !entity.is_silent() {
        level.level_event(1025, entity.block_position(), 0);
    }
}

pub(super) fn tick_bat_ambient_sound(entity: &Entity, store: &mut EntityStore) {
    let Some(resting) = (match entity.data() {
        EntityData::GeneralLiving(data) => data.bat_retained().map(BatRetainedState::is_resting),
        _ => None,
    }) else {
        return;
    };
    let alive = entity.is_alive() && entity.living().is_some_and(|living| living.health() > 0.0);
    let pitch = store
        .mob_ai_mut(entity.id())
        .and_then(|runtime| runtime.tick_bat_ambient_sound(alive, resting));
    let Some(pitch) = pitch else {
        return;
    };
    if !entity.is_silent() {
        store.play_entity_sound(PendingEntitySound {
            position: entity.position(),
            resource_id: "minecraft:entity.bat.ambient".to_string(),
            source: SoundSource::Neutral,
            volume: crate::entity::ai::mob::bat::AMBIENT_VOLUME,
            pitch,
        });
    }
}

pub(super) fn tick_bat_after_super(entity: &mut Entity) {
    let Some(resting) = (match entity.data() {
        EntityData::GeneralLiving(data) => data.bat_retained().map(BatRetainedState::is_resting),
        _ => None,
    }) else {
        return;
    };
    if resting {
        entity.set_delta_movement(Vec3d::default());
        let position = entity.position();
        entity.set_pos(Vec3d {
            y: position.y.floor() + 1.0 - f64::from(entity.bb_height()),
            ..position
        });
    } else {
        let movement = entity.delta_movement();
        entity.set_delta_movement(Vec3d {
            y: movement.y * 0.6,
            ..movement
        });
    }
}
