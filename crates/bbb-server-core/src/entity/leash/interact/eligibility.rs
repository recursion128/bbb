//! Player-facing leash attachment eligibility.

use crate::entity::ai::mob::spec::is_instance_of;
use crate::entity::ai::MemoryModuleType;
use crate::entity::store::EntityStore;
use crate::entity::{Entity, EntityData};
use bbb_protocol::packets::Vec3d;

/// `Mob.canBeLeashed` (`Mob.java:1263-1265`) and its subclass overrides.
pub(super) fn can_be_leashed(entity: &Entity, store: &EntityStore) -> bool {
    if crate::entity::boat::is_boat(entity.data()) {
        return true;
    }
    let EntityData::GeneralLiving(data) = entity.data() else {
        return false;
    };
    if !data.is_mob() {
        return false;
    }
    let kind = entity.entity_type_id();
    let instance = |class| is_instance_of(kind, class);
    let mob_controlled = || {
        store.passengers(entity.id()).first()
        .and_then(|id| store.get(*id))
        .is_some_and(|passenger| matches!(passenger.data(), EntityData::GeneralLiving(data) if data.is_mob()))
    };
    if instance("Wolf") {
        return !crate::entity::ai::mob::target::anger::read(entity)
            .is_some_and(|anger| anger.is_angry(store.game_time()));
    }
    if instance("AbstractNautilus") {
        // `AbstractNautilus.isAggravated` (`AbstractNautilus.java:528-529`).
        let live_anger = store
            .mob_ai(entity.id())
            .and_then(|runtime| runtime.nautilus_is_angry(entity.tick_count()));
        let angry = live_anger.unwrap_or_else(|| {
            crate::entity::general_living::spawner_brain_memory(entity, MemoryModuleType::AngryAt)
                .is_some_and(|memory| memory.time_to_live.is_none_or(|ttl| ttl > 0))
        });
        let attack_target = store.mob_target(entity.id()).is_some()
            || (live_anger.is_none()
                && crate::entity::general_living::spawner_brain_memory(
                    entity,
                    MemoryModuleType::AttackTarget,
                )
                .is_some());
        return !angry && !attack_target && (!instance("ZombieNautilus") || !mob_controlled());
    }
    if instance("ZombieHorse") {
        return data
            .abstract_horse_retained()
            .is_some_and(|horse| horse.is_tame())
            || !mob_controlled();
    }
    if instance("CamelHusk") {
        return !mob_controlled();
    }
    if [
        "TamableAnimal",
        "Hoglin",
        "Zoglin",
        "Axolotl",
        "Dolphin",
        "Squid",
    ]
    .into_iter()
    .any(instance)
    {
        return true;
    }
    if [
        "AmbientCreature",
        "AbstractVillager",
        "Panda",
        "Turtle",
        "WaterAnimal",
        "AgeableWaterCreature",
    ]
    .into_iter()
    .any(instance)
    {
        return false;
    }
    !crate::entity::ai::entity_type_is_enemy(kind)
}

/// `Leashable.canHaveALeashAttachedTo` (`Leashable.java:55-61`).
pub(super) fn can_attach(
    entity: &Entity,
    store: &EntityStore,
    holder_id: i32,
    center: Vec3d,
) -> bool {
    entity.id() != holder_id
        && super::super::distance_to(center, entity.bounding_box().center())
            <= super::super::leash_snap_distance(entity)
        && can_be_leashed(entity, store)
}
