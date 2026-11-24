//! `MaceItem` smash rules shared by player and mob attackers (`MaceItem.java:51-156`).

use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID;
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::item_types::VANILLA_ITEM_MACE_ID;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::geometry::{Aabb, Vec3Ops};

use crate::entity::{Entity, EntityData, EntityStore};
use crate::world::simulation_level::LevelCtx;

pub(crate) const MACE_SMASH_DAMAGE_TYPE: &str = "minecraft:mace_smash";
pub(crate) const MACE_SMASH_AIR_SOUND: &str = "minecraft:item.mace.smash_air";
pub(crate) const MACE_SMASH_GROUND_SOUND: &str = "minecraft:item.mace.smash_ground";
pub(crate) const MACE_SMASH_GROUND_HEAVY_SOUND: &str = "minecraft:item.mace.smash_ground_heavy";
/// `LevelEvent.PARTICLES_SMASH_ATTACK`, sent with 750 particles.
pub(crate) const LEVEL_EVENT_SMASH_ATTACK: i32 = 2013;

/// `MaceItem.canSmashAttack` (`MaceItem.java:149-151`).
pub(crate) fn can_smash_attack(fall_distance: f64, fall_flying: bool) -> bool {
    fall_distance > 1.5 && !fall_flying
}

/// `MaceItem.getAttackDamageBonus` without its per-block enchantment term (`:89-113`).
pub(crate) fn smash_damage(fall_distance: f64) -> f64 {
    if fall_distance <= 3.0 {
        4.0 * fall_distance
    } else if fall_distance <= 8.0 {
        12.0 + 2.0 * (fall_distance - 3.0)
    } else {
        22.0 + fall_distance - 8.0
    }
}

/// A mob swinging a mace it can smash with.
pub(crate) fn mob_can_smash(entity: &Entity) -> bool {
    entity
        .living()
        .and_then(|living| living.item_by_slot(EquipmentSlot::MainHand))
        .is_some_and(|stack| stack.item_id() == Some(VANILLA_ITEM_MACE_ID))
        && can_smash_attack(
            entity.fall_distance(),
            entity.get_shared_flag(VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING),
        )
}

/// What a mob's `hurtEnemy` reads off an online player.
pub(crate) struct MaceSessionFacts {
    pub on_ground: bool,
    pub on_pos: BlockPos,
    pub creative_flying: bool,
    pub knockback_resistance: f64,
}

/// `MaceItem.hurtEnemy` (`MaceItem.java:51-73`) with a mob attacker.
pub(crate) fn mob_hurt_enemy(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    target_id: i32,
) {
    if !mob_can_smash(entity) {
        return;
    }
    let fall_distance = entity.fall_distance();
    let heavy = fall_distance > 5.0;
    let target = match store.get(target_id) {
        Some(target) => Some((
            target.on_ground(),
            target.get_on_pos(),
            target.bounding_box(),
            target.position(),
            target.uuid(),
        )),
        None => level.player_by_entity_id(target_id).and_then(|player| {
            let facts = store.player(target_id)?.mace_facts();
            Some((
                facts.on_ground,
                facts.on_pos,
                Aabb::of_entity(player.position, player.dimensions[0], player.dimensions[1]),
                player.position,
                player.uuid,
            ))
        }),
    };
    let Some((on_ground, on_pos, target_box, target_position, target_uuid)) = target else {
        return;
    };
    let mut delta = entity.delta_movement();
    delta.y = f64::from(0.01_f32);
    entity.set_delta_movement(delta);
    let position = entity.position();
    if let Some(living) = entity.living_mut() {
        // `calculateImpactPosition` (`:75-79`).
        let impact = match living.fall_impulse.impact_position {
            Some(impact) if impact.y <= position.y => impact,
            _ => position,
        };
        living.fall_impulse.set_ignore_fall_damage(true, impact);
    }
    let sound = match (on_ground, heavy) {
        (true, true) => MACE_SMASH_GROUND_HEAVY_SOUND,
        (true, false) => MACE_SMASH_GROUND_SOUND,
        (false, _) => MACE_SMASH_AIR_SOUND,
    };
    store.play_entity_sound(crate::entity::PendingEntitySound {
        position,
        resource_id: sound.to_owned(),
        source: crate::entity::living::living_sound_source(entity),
        volume: 1.0,
        pitch: 1.0,
    });
    level.level_event(LEVEL_EVENT_SMASH_ATTACK, on_pos, 750);

    // `MaceItem.knockback` (`:115-147`).
    let area = target_box.inflate(3.5, 3.5, 3.5);
    let power = |nearby: Vec3d, resistance: f64| {
        let direction = nearby.plus(target_position.scale(-1.0));
        let power = (3.5 - direction.length_sqr().sqrt())
            * f64::from(0.7_f32)
            * if heavy { 2.0 } else { 1.0 }
            * (1.0 - resistance);
        let unit = direction.normalize();
        (power, unit.x * power, unit.z * power)
    };
    let in_reach = |nearby: Vec3d| target_position.distance_to_sqr(nearby) <= 3.5 * 3.5;
    let mut stored = Vec::new();
    let mut sessions = Vec::new();
    {
        let sensing = Default::default();
        let view = crate::entity::ai::mob::target::TargetView::new(entity, store, level, &sensing);
        let attacker = view.body();
        let allied = |id: i32| {
            view.facts(id).is_some_and(|nearby| {
                crate::entity::ai::mob::target::conditions::is_allied_to(&view, &attacker, &nearby)
            })
        };
        for id in store.ids_in_box_sections(area) {
            let Some(nearby) = store.get(id) else {
                continue;
            };
            if id == entity.id()
                || id == target_id
                || nearby.living().is_none()
                || !nearby.bounding_box().intersects(area)
                || !in_reach(nearby.position())
                || matches!(nearby.tamable_owner(), Some((true, Some(owner))) if owner == target_uuid)
                || matches!(nearby.data(), EntityData::ArmorStand(data) if data.is_marker())
                || allied(id)
            {
                continue;
            }
            let resistance = nearby.living().map_or(0.0, |living| {
                living.attribute_value(VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID)
            });
            stored.push((id, power(nearby.position(), resistance)));
        }
        for player in level.players_in_box(area) {
            let Some(facts) = store
                .player(player.entity_id)
                .map(|state| state.mace_facts())
            else {
                continue;
            };
            if player.spectator
                || player.entity_id == target_id
                || facts.creative_flying
                || !in_reach(player.position)
                || allied(player.entity_id)
            {
                continue;
            }
            sessions.push((
                player.entity_id,
                power(player.position, facts.knockback_resistance),
            ));
        }
    }
    for (id, (power, x, z)) in stored {
        if power > 0.0 {
            if let Some(nearby) = store.get_mut(id) {
                crate::entity::push(nearby, x, f64::from(0.7_f32), z);
            }
        }
    }
    for (id, (power, x, z)) in sessions {
        if power > 0.0 && x.is_finite() && z.is_finite() {
            if let Some(player) = store.player_mut(id) {
                player.push_impulse(Vec3d {
                    x,
                    y: f64::from(0.7_f32),
                    z,
                });
            }
        }
    }
}
