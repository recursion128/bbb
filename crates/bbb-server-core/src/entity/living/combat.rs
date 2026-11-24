//! Living entity combat formulas: knockback, armor absorption, invulnerability, and hurt cooldown.

use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_ARMOR_ID, VANILLA_ATTRIBUTE_ARMOR_TOUGHNESS_ID,
    VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID,
};
use bbb_protocol::packets::{ItemStackSummary, Vec3d};
use bbb_server_datapack::damage::{DamageTypeFacts, HURT_DURATION};
use bbb_server_types::geometry::Vec3Ops;

use crate::entity::store::EntityStore;
use crate::entity::Entity;

use super::{LivingEntityState, INVULNERABLE_DURATION};

pub fn scaled_knockback_power(power: f64, resistance: f64) -> Option<f64> {
    let scaled = power * (1.0 - resistance);
    (scaled > 0.0).then_some(scaled)
}

pub fn active_main_hand_disable_blocking_seconds(
    using_off_hand: bool,
    main_hand: Option<&ItemStackSummary>,
) -> std::result::Result<f32, String> {
    if using_off_hand {
        return Ok(0.0);
    }
    let Some(weapon) = main_hand else {
        return Ok(0.0);
    };
    Ok(crate::world::using_item::weapon_disable_blocking_seconds(weapon)?.unwrap_or(0.0))
}

pub fn hurt_knockback_direction(
    position: Vec3d,
    projectile_motion: Option<Vec3d>,
    projectile_origin: Option<Vec3d>,
    source_position: Option<Vec3d>,
) -> (f64, f64) {
    if let Some(motion) = projectile_motion {
        (-motion.x, -motion.z)
    } else if let Some(origin) = projectile_origin {
        (-(position.x - origin.x), -(position.z - origin.z))
    } else if let Some(source) = source_position {
        (source.x - position.x, source.z - position.z)
    } else {
        (0.0, 0.0)
    }
}

/// `LivingEntity.knockback` (`LivingEntity.java:1614-1632`).
///
/// The `while` at `:1618-1621` redraws the direction off the entity's stream
/// while it is degenerate; the draws are the store's here, in the same order
/// and count.
pub fn knockback(entity: &mut Entity, store: &mut EntityStore, power: f64, xd: f64, zd: f64) {
    let Some(living) = entity.living() else {
        return;
    };
    // `EnderDragon.knockback` (`EnderDragon.java:488-492`).
    if crate::entity::ender_dragon::state(entity).is_some_and(|dragon| dragon.is_sitting()) {
        return;
    }
    let Some(power) = scaled_knockback_power(
        power,
        living.attribute_value(VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID),
    ) else {
        return;
    };
    entity.set_needs_sync(true);
    let delta = knockback_delta(
        entity.delta_movement(),
        entity.on_ground(),
        power,
        xd,
        zd,
        || store.random().next_double(),
    );
    entity.set_delta_movement(delta);
}

pub fn knockback_delta(
    delta: Vec3d,
    on_ground: bool,
    power: f64,
    mut xd: f64,
    mut zd: f64,
    mut next_double: impl FnMut() -> f64,
) -> Vec3d {
    while xd * xd + zd * zd < f64::from(1.0e-5_f32) {
        xd = (next_double() - next_double()) * 0.01;
        zd = (next_double() - next_double()) * 0.01;
    }
    let vector = Vec3d {
        x: xd,
        y: 0.0,
        z: zd,
    }
    .normalize()
    .scale(power);
    let y = if on_ground {
        (delta.y / 2.0 + power).min(0.4)
    } else {
        delta.y
    };
    Vec3d {
        x: delta.x / 2.0 - vector.x,
        y,
        z: delta.z / 2.0 - vector.z,
    }
}

pub fn damage_after_absorb(damage: f32, total_armor: f32, armor_toughness: f32) -> f32 {
    let toughness = 2.0 + armor_toughness / 4.0;
    let real_armor = clamp_f32(total_armor - damage / toughness, total_armor * 0.2, 20.0);
    let armor_fraction = real_armor / 25.0;
    damage * (1.0 - armor_fraction)
}

pub fn clamp_f32(value: f32, min: f32, max: f32) -> f32 {
    if value < min {
        min
    } else {
        value.min(max)
    }
}

pub fn armor_value(living: &LivingEntityState) -> i32 {
    living.attribute_value(VANILLA_ATTRIBUTE_ARMOR_ID).floor() as i32
}

/// `LivingEntity.actuallyHurt` (`LivingEntity.java:1921-1944`).
pub fn actually_hurt(living: &mut LivingEntityState, bypasses_armor: bool, damage: f32) -> f32 {
    let mut damage = damage;
    if !bypasses_armor {
        damage = damage_after_absorb(
            damage,
            armor_value(living) as f32,
            living.attribute_value(VANILLA_ATTRIBUTE_ARMOR_TOUGHNESS_ID) as f32,
        );
    }
    if damage <= 0.0 {
        damage = 0.0;
    }
    let original = damage;
    damage = (damage - living.absorption_amount()).max(0.0);
    let absorption = living.absorption_amount();
    living.set_absorption_amount(absorption - (original - damage));
    if damage != 0.0 {
        let health = living.health();
        living.set_health(health - damage);
        let absorption = living.absorption_amount();
        living.set_absorption_amount(absorption - damage);
    }
    damage
}

pub fn is_invulnerable_to(
    entity: &Entity,
    damage_type: &DamageTypeFacts,
    creative_player: bool,
    fall_damage_immune: bool,
) -> bool {
    is_invulnerable_to_with_tags(
        entity,
        creative_player,
        fall_damage_immune,
        damage_type.bypasses_invulnerability,
        damage_type.is_fire,
        damage_type.is_fall,
    )
}

pub(crate) fn is_invulnerable_to_with_tags(
    entity: &Entity,
    creative_player: bool,
    fall_damage_immune: bool,
    bypasses_invulnerability: bool,
    is_fire: bool,
    is_fall: bool,
) -> bool {
    entity.removal_reason().is_some()
        // Warden.java:160-161.
        || crate::entity::warden::is_digging_or_emerging(entity.entity_type_id(), entity.pose_id())
            && !bypasses_invulnerability
        || entity.invulnerable() && !bypasses_invulnerability && !creative_player
        || is_fire && entity.fire_immune()
        || is_fall && fall_damage_immune
}

/// Returns `None` when the hit is swallowed (`damage <= lastHurt` inside the
/// window, `:1197-1199`), otherwise the amount to deal and whether the entity
/// `tookFullDamage`.
pub fn hurt_cooldown(
    living: &mut LivingEntityState,
    bypasses_cooldown: bool,
    damage: f32,
) -> Option<(f32, bool)> {
    // `:1196`: `invulnerableTime > 10.0F`, an int against a float.
    if living.invulnerable_time as f32 > 10.0 && !bypasses_cooldown {
        if damage <= living.last_hurt {
            return None;
        }
        let dealt = damage - living.last_hurt;
        living.last_hurt = damage;
        Some((dealt, false))
    } else {
        living.last_hurt = damage;
        living.invulnerable_time = INVULNERABLE_DURATION;
        living.hurt_duration = HURT_DURATION;
        living.hurt_time = living.hurt_duration;
        Some((damage, true))
    }
}

pub fn mark_dead(entity: &mut Entity) {
    if let Some(living) = entity.living_mut() {
        living.dead = true;
    }
}
