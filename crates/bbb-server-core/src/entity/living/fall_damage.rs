use super::LivingEntityState;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_FALL_DAMAGE_MULTIPLIER_ID, VANILLA_ATTRIBUTE_SAFE_FALL_DISTANCE_ID,
};
use bbb_protocol::block_sound_facts_26_1::vanilla_block_state_sound_type_26_1;
use bbb_protocol::codec::write_var_i32_to;
use bbb_protocol::packets::{BlockPos, LevelParticles, ParticlePayload, SoundSource, Vec3d};
use bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_BLOCK_ID;

use crate::entity::store::{EntityStore, PendingEntitySound};
use crate::entity::Entity;

fn calculate_fall_power(fall_distance: f64, safe_fall_distance: f64) -> f64 {
    fall_distance + 1.0e-6 - safe_fall_distance
}

pub fn body_fall_particles(
    position: Vec3d,
    support: BlockPos,
    state: i32,
    distance: f64,
    safe_distance: f64,
) -> Option<LevelParticles> {
    let power = (calculate_fall_power(distance, safe_distance).floor() as i32).max(0);
    if power == 0 || bbb_server_types::fluid::block_state_is_air(state) {
        return None;
    }
    let mut origin = position;
    if support.x != position.x.floor() as i32 || support.z != position.z.floor() as i32 {
        let x_diff = position.x - f64::from(support.x) - 0.5;
        let z_diff = position.z - f64::from(support.z) - 0.5;
        let max_diff = x_diff.abs().max(z_diff.abs());
        origin.x = f64::from(support.x) + 0.5 + x_diff / max_diff * 0.5;
        origin.z = f64::from(support.z) + 0.5 + z_diff / max_diff * 0.5;
    }
    let scale = (f64::from(0.2_f32) + f64::from(power) / 15.0).min(2.5);
    let mut raw_options = Vec::new();
    write_var_i32_to(&mut raw_options, state);
    Some(LevelParticles {
        override_limiter: false,
        always_show: false,
        position: origin,
        offset: Vec3d::default(),
        max_speed: 0.15_f32,
        count: (150.0 * scale) as i32,
        particle: ParticlePayload {
            particle_type_id: VANILLA_PARTICLE_TYPE_BLOCK_ID,
            raw_options,
        },
    })
}

///
/// `fall_damage_immune` is the entity type's membership of
/// `#minecraft:fall_damage_immune`, which the caller answers from the
/// generated registry facts.
pub fn calculate_fall_damage(
    living: &LivingEntityState,
    fall_distance: f64,
    damage_modifier: f32,
    fall_damage_immune: bool,
) -> i32 {
    calculate_body_fall_damage(
        fall_distance,
        damage_modifier,
        living.attribute_value(VANILLA_ATTRIBUTE_SAFE_FALL_DISTANCE_ID),
        living.attribute_value(VANILLA_ATTRIBUTE_FALL_DAMAGE_MULTIPLIER_ID),
        fall_damage_immune,
    )
}

pub fn calculate_body_fall_damage(
    fall_distance: f64,
    damage_modifier: f32,
    safe_fall_distance: f64,
    fall_damage_multiplier: f64,
    fall_damage_immune: bool,
) -> i32 {
    if fall_damage_immune {
        return 0;
    }
    let base = calculate_fall_power(fall_distance, safe_fall_distance);
    (base * f64::from(damage_modifier) * fall_damage_multiplier).floor() as i32
}

/// `SoundEvents.GENERIC_SMALL_FALL`/`GENERIC_BIG_FALL`, the
/// `LivingEntity.getFallSounds()` default (`LivingEntity.java:1680-1682`).
pub const GENERIC_SMALL_FALL_SOUND: &str = "minecraft:entity.generic.small_fall";
pub const GENERIC_BIG_FALL_SOUND: &str = "minecraft:entity.generic.big_fall";

pub(crate) fn fall_sound_for(entity: &Entity, big: bool) -> &'static str {
    if matches!(entity.data(), crate::entity::EntityData::ArmorStand(_)) {
        return "minecraft:entity.armor_stand.fall";
    }
    let monster = crate::entity::ai::mob::spec::is_instance_of(entity.entity_type_id(), "Monster");
    match (monster, big) {
        (true, true) => "minecraft:entity.hostile.big_fall",
        (true, false) => "minecraft:entity.hostile.small_fall",
        (false, true) => GENERIC_BIG_FALL_SOUND,
        (false, false) => GENERIC_SMALL_FALL_SOUND,
    }
}

pub fn fall_damage_sound(damage: i32, small: &'static str, big: &'static str) -> &'static str {
    if damage > 4 {
        big
    } else {
        small
    }
}

pub(crate) fn fall_damage_sounds(
    entity: &Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    damage_sound: &'static str,
) {
    if entity.is_silent() {
        return;
    }
    let position = entity.position();
    let source = super::living_sound_source(entity);
    store.play_entity_sound(PendingEntitySound {
        position,
        resource_id: damage_sound.to_owned(),
        source,
        volume: 1.0,
        pitch: 1.0,
    });
    if let Some(sound) = body_block_fall_sound(position, source, |pos| level.block_state(pos)) {
        store.play_entity_sound(sound);
    }
}

pub fn body_block_fall_sound(
    position: Vec3d,
    source: SoundSource,
    block_state: impl FnOnce(BlockPos) -> i32,
) -> Option<PendingEntitySound> {
    // `Mth.floor(getY() - 0.2F)` at `float` precision, widened for the
    // subtraction as Java widens `0.2F`.
    let block_pos = BlockPos {
        x: position.x.floor() as i32,
        y: (position.y - f64::from(0.2_f32)).floor() as i32,
        z: position.z.floor() as i32,
    };
    let block_state = block_state(block_pos);
    if bbb_server_types::fluid::block_state_is_air(block_state) {
        return None;
    }
    let sound_type = vanilla_block_state_sound_type_26_1(block_state);
    Some(PendingEntitySound {
        position,
        resource_id: sound_type.fall_sound.to_owned(),
        source,
        volume: sound_type.volume * 0.5,
        pitch: sound_type.pitch * 0.75,
    })
}

#[cfg(test)]
mod tests;

mod passengers;
pub(crate) use passengers::{cause_fall_damage, propagate_fall_to_passengers};
