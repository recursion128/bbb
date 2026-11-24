//! `Ravager`'s attack, stun and roar counters (`Ravager.java:61-63`) and the
//! `aiStep` that runs them (`Ravager.java:133-183`).

use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_RAVAGER_ID,
};
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::geometry::Aabb;

use crate::connection::ClientboundPlayPacket;
use crate::entity::save::optional_int;
use crate::entity::{Entity, EntityData, EntityStore};
use crate::random::GameRandom;
use crate::world::simulation_level::LevelCtx;

/// `Ravager.ATTACK_DURATION` and `STUN_DURATION` (`Ravager.java:56-57`).
const ATTACK_DURATION: i32 = 10;
const STUN_DURATION: i32 = 40;
/// The entity events `blockedByItem` and `roar` broadcast (`Ravager.java:212`, `:241`).
const STUNNED_ENTITY_EVENT: i8 = 39;
const ROAR_ENTITY_EVENT: i8 = 69;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RavagerTicks {
    attack: i32,
    stun: i32,
    roar: i32,
}

impl RavagerTicks {
    pub(crate) const DEFAULT: Self = Self {
        attack: 0,
        stun: 0,
        roar: 0,
    };

    /// `Ravager.isImmobile`'s own terms (`Ravager.java:196-198`).
    const fn immobile(self) -> bool {
        self.attack > 0 || self.stun > 0 || self.roar > 0
    }
}

pub(crate) fn owns_save_field(name: &str) -> bool {
    matches!(name, "AttackTick" | "StunTick" | "RoarTick")
}

pub(crate) fn write_save_data(entries: &mut Vec<NbtSummaryEntry>, ticks: RavagerTicks) {
    for (name, value) in [
        ("AttackTick", ticks.attack),
        ("StunTick", ticks.stun),
        ("RoarTick", ticks.roar),
    ] {
        entries.push(NbtSummaryEntry {
            name: name.into(),
            value: NbtSummaryValue::Int(value),
        });
    }
}

pub(crate) fn read_save_data(entries: &[NbtSummaryEntry]) -> RavagerTicks {
    RavagerTicks {
        attack: optional_int(entries, "AttackTick").unwrap_or(0),
        stun: optional_int(entries, "StunTick").unwrap_or(0),
        roar: optional_int(entries, "RoarTick").unwrap_or(0),
    }
}

pub(crate) fn ticks(entity: &Entity) -> Option<RavagerTicks> {
    match entity.data() {
        EntityData::GeneralLiving(data) => data.raider_retained()?.ravager(),
        _ => None,
    }
}

fn ticks_mut(entity: &mut Entity) -> Option<&mut RavagerTicks> {
    match &mut entity.data {
        EntityData::GeneralLiving(data) => data.raider_retained_mut()?.ravager_mut(),
        _ => None,
    }
}

/// `Ravager.doHurtTarget`'s `attackTick = 10` (`Ravager.java:293`).
pub(crate) fn begin_attack(entity: &mut Entity) {
    if let Some(ticks) = ticks_mut(entity) {
        ticks.attack = ATTACK_DURATION;
    }
}

/// `Ravager.isImmobile` beyond the living base.
pub(crate) fn is_immobile(entity: &Entity) -> bool {
    ticks(entity).is_some_and(RavagerTicks::immobile)
}

/// `Ravager.blockedByItem` (`Ravager.java:206-219`) against a defender at
/// `defender`: a stun pushes the two apart (`Entity.push(Entity)`,
/// `Entity.java:1782-1811`), otherwise `strongKnockback` throws the
/// defender. Answers the defender's push, `None` while roaring.
pub(crate) fn blocked_by_item(
    entity: &mut Entity,
    store: &mut EntityStore,
    random: &GameRandom,
    defender: Vec3d,
) -> Option<Vec3d> {
    let ticks = ticks_mut(entity)?;
    if ticks.roar != 0 {
        return None;
    }
    if random.next_double() >= 0.5 {
        return Some(strong_knockback(entity.position(), defender));
    }
    ticks.stun = STUN_DURATION;
    play_sound(entity, store, "minecraft:entity.ravager.stunned");
    broadcast_event(entity, store, STUNNED_ENTITY_EVENT);
    let (xa, za) =
        crate::entity::push::pair_impulse(defender, entity.position()).unwrap_or((0.0, 0.0));
    crate::entity::push::push(entity, xa, 0.0, za);
    Some(Vec3d {
        x: -xa,
        y: 0.0,
        z: -za,
    })
}

/// `Ravager.strongKnockback`'s push on `target` (`Ravager.java:250-255`).
fn strong_knockback(ravager: Vec3d, target: Vec3d) -> Vec3d {
    let xd = target.x - ravager.x;
    let zd = target.z - ravager.z;
    let dd = (xd * xd + zd * zd).max(0.001);
    Vec3d {
        x: xd / dd * 4.0,
        y: 0.2,
        z: zd / dd * 4.0,
    }
}

/// `Ravager.aiStep` past `super.aiStep()` (`Ravager.java:133-183`), without
/// the stun particles a server level never shows.
pub(crate) fn after_ai_step(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    let Some(ticks) = ticks(entity) else {
        return true;
    };
    let alive = entity.is_alive()
        && entity
            .living()
            .is_some_and(|living| !living.is_dead_or_dying());
    if !alive {
        return true;
    }
    let immobile = ticks.immobile();
    let has_target = store
        .mob_ai(entity.id())
        .is_some_and(crate::entity::ai::mob::MobAiRuntime::has_target);
    if let Some(speed) = entity.living_mut().and_then(|living| {
        living
            .attributes_mut()
            .get_instance(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
    }) {
        let base = speed.base_value();
        let next = if immobile {
            0.0
        } else {
            let max_speed = if has_target { 0.35 } else { 0.3 };
            base + 0.1 * (max_speed - base)
        };
        speed.set_base_value(next);
    }
    if entity.horizontal_collision() && level.mob_griefing() {
        break_leaves_or_jump(entity, store, level);
    }
    let mut roar_now = false;
    let mut roar_sound = false;
    if let Some(ticks) = ticks_mut(entity) {
        if ticks.roar > 0 {
            ticks.roar -= 1;
            roar_now = ticks.roar == 10;
        }
        if ticks.attack > 0 {
            ticks.attack -= 1;
        }
        if ticks.stun > 0 {
            ticks.stun -= 1;
            if ticks.stun == 0 {
                ticks.roar = 20;
                roar_sound = true;
            }
        }
    }
    if roar_now && !roar(entity, store, level) {
        return false;
    }
    if roar_sound {
        play_sound(entity, store, "minecraft:entity.ravager.roar");
    }
    true
}

// Ravager.java:144-160.
fn break_leaves_or_jump(entity: &mut Entity, store: &EntityStore, level: &mut LevelCtx<'_, '_>) {
    let bounds = entity.bounding_box().inflate(0.2, 0.2, 0.2);
    let mut destroyed = false;
    for x in bounds.min_x.floor() as i32..=bounds.max_x.floor() as i32 {
        for y in bounds.min_y.floor() as i32..=bounds.max_y.floor() as i32 {
            for z in bounds.min_z.floor() as i32..=bounds.max_z.floor() as i32 {
                let pos = BlockPos { x, y, z };
                if level.block_state_in_tag(level.block_state(pos), "minecraft:leaves") {
                    destroyed |= level.destroy_block_by_entity(pos, true, entity, Some(store));
                }
            }
        }
    }
    if !destroyed && entity.on_ground() {
        crate::entity::living::jump_from_ground(entity, level);
    }
}

/// `Ravager.roar` (`Ravager.java:221-238`).
fn roar(entity: &mut Entity, store: &mut EntityStore, level: &mut LevelCtx<'_, '_>) -> bool {
    let reach = entity.bounding_box().inflate(4.0, 4.0, 4.0);
    let griefing = level.mob_griefing();
    let position = entity.position();
    for player in level.players().to_vec() {
        let bounds = Aabb::of_entity(player.position, player.dimensions[0], player.dimensions[1]);
        if player.dead_or_dying || player.spectator || !bounds.intersects(reach) {
            continue;
        }
        let attacker = crate::level_event::PendingHurtEntity::Stored {
            entity_id: entity.id(),
        };
        level.hurt_player(crate::level_event::PendingPlayerHurt {
            player: player.uuid,
            damage_type: crate::entity::ai::MOB_ATTACK_DAMAGE_TYPE,
            amount: 6.0,
            causing: Some(attacker.clone()),
            direct: Some(attacker),
            post_attack: false,
            impulse: None,
        });
    }
    for target_id in store.ids_in_box_sections(reach) {
        let Some(target) = store.get(target_id) else {
            continue;
        };
        let target_type = target.entity_type_id();
        let selected = target.living().is_some()
            && target.is_alive()
            && target.bounding_box().intersects(reach)
            && target_type != VANILLA_ENTITY_TYPE_RAVAGER_ID
            && (griefing || target_type != VANILLA_ENTITY_TYPE_ARMOR_STAND_ID);
        if !selected {
            continue;
        }
        if !crate::entity::ai::mob::spec::is_instance_of(target_type, "AbstractIllager")
            && crate::entity::ai::mob::brain_runtime::hurt_entity_from_body(
                entity,
                store,
                level,
                target_id,
                crate::entity::ai::MOB_ATTACK_DAMAGE_TYPE,
                6.0,
            )
            .is_none()
        {
            return false;
        }
        if let Some(mut target) = store.detach(target_id) {
            let push = strong_knockback(position, target.position());
            crate::entity::push::push(&mut target, push.x, push.y, push.z);
            store.attach(target);
        }
    }
    level.game_event_at(
        crate::level_event::VanillaGameEvent::ENTITY_ACTION,
        position,
        Some(entity.id()),
    );
    broadcast_event(entity, store, ROAR_ENTITY_EVENT);
    true
}

fn broadcast_event(entity: &Entity, store: &mut EntityStore, event_id: i8) {
    store.broadcast_to_tracking(
        entity.id(),
        ClientboundPlayPacket::EntityEvent(bbb_protocol::packets::EntityEvent {
            entity_id: entity.id(),
            event_id,
        }),
    );
}

fn play_sound(entity: &Entity, store: &mut EntityStore, resource_id: &str) {
    if entity.is_silent() {
        return;
    }
    store.play_entity_sound(crate::entity::PendingEntitySound {
        position: entity.position(),
        resource_id: resource_id.to_owned(),
        source: crate::entity::living::living_sound_source(entity),
        volume: 1.0,
        pitch: 1.0,
    });
}

#[cfg(test)]
mod tests;
