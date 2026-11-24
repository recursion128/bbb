//! `AbstractHorse`'s per-tick steps and its stand and ambient sounds.

use super::*;
use crate::world::simulation_level::LevelCtx;

pub(super) fn tick_abstract_horse_pre_ai(entity: &mut Entity, store: &mut EntityStore) {
    let is_horse = matches!(
        entity.data(),
        EntityData::GeneralLiving(data) if data.abstract_horse_retained().is_some()
    );
    if !is_horse
        || store
            .mob_ai_mut(entity.id())
            .is_none_or(|runtime| runtime.next_random_int(200) != 0)
    {
        return;
    }
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        data.abstract_horse_retained_mut()
            .expect("classified AbstractHorse retained state")
            .move_tail();
    }
}

pub(super) fn tick_abstract_horse_post_ai(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
) {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return;
    };
    let Some(horse) = data.abstract_horse_retained() else {
        return;
    };
    let can_eat_grass = !matches!(
        entity.entity_type_id(),
        VANILLA_ENTITY_TYPE_LLAMA_ID | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID
    );
    let draws_grass = can_eat_grass && !horse.is_eating();
    let Some(runtime) = store.mob_ai_mut(entity.id()) else {
        return;
    };
    let heals = runtime.next_random_int(900) == 0
        && entity.living().is_some_and(|living| living.death_time == 0);
    let grass_draw = draws_grass.then(|| runtime.next_random_int(300) == 0);
    if heals {
        let health = entity.living().map_or(0.0, LivingEntityState::health);
        living::set_health(entity, health + 1.0);
    }
    let starts_eating = grass_draw == Some(true) && is_grass_below(entity, level);
    let changed = match &mut entity.data {
        EntityData::GeneralLiving(data) => data
            .abstract_horse_retained_mut()
            .expect("classified AbstractHorse retained state")
            .post_ai_step(can_eat_grass, starts_eating),
        _ => false,
    };
    if changed {
        entity.mark_synched_dirty(VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID);
    }
}

fn is_grass_below(entity: &Entity, level: &LevelCtx<'_, '_>) -> bool {
    let pos = entity.block_position();
    vanilla_block_state_block_type_id(level.block_state(bbb_protocol::packets::BlockPos {
        y: pos.y - 1,
        ..pos
    })) == Some(VANILLA_BLOCK_GRASS_BLOCK_ID)
}

pub(super) fn tick_abstract_horse_counters(entity: &mut Entity) {
    let changed = match &mut entity.data {
        EntityData::GeneralLiving(data) => data
            .abstract_horse_retained_mut()
            .is_some_and(AbstractHorseRetainedState::tick_counters),
        _ => false,
    };
    if changed {
        entity.mark_synched_dirty(VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID);
    }
}

pub(crate) fn apply_abstract_horse_random_stand(entity: &mut Entity, store: &mut EntityStore) {
    let changed = match &mut entity.data {
        EntityData::GeneralLiving(data) => data
            .abstract_horse_retained_mut()
            .map(|horse| horse.set_standing(20)),
        _ => None,
    };
    if changed == Some(true) {
        entity.mark_synched_dirty(VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID);
    }
    if changed.is_none() {
        return;
    }
    if entity.is_silent() {
        return;
    }
    if let Some(resource_id) = abstract_horse_ambient_sound(entity) {
        store.play_entity_sound(PendingEntitySound {
            position: entity.position(),
            resource_id: resource_id.to_string(),
            source: SoundSource::Neutral,
            volume: 1.0,
            pitch: 1.0,
        });
    }
}

pub(crate) fn tick_abstract_horse_ambient_sound(entity: &Entity, store: &mut EntityStore) {
    if !is_abstract_horse_entity_type(entity.entity_type_id()) {
        return;
    }
    let is_baby = matches!(entity.data(), EntityData::GeneralLiving(data)
        if data.ageable_retained().is_some_and(AgeableMobRetainedState::is_baby));
    let alive = entity.is_alive() && entity.living().is_some_and(|living| living.health() > 0.0);
    let triggered = store
        .mob_ai_mut(entity.id())
        .is_some_and(|runtime| runtime.tick_abstract_horse_ambient_sound(alive));
    if !triggered {
        return;
    }
    let Some(resource_id) = abstract_horse_ambient_sound(entity) else {
        return;
    };
    let Some(pitch) = store
        .mob_ai_mut(entity.id())
        .map(|runtime| runtime.voice_pitch(is_baby))
    else {
        return;
    };
    if !entity.is_silent() {
        store.play_entity_sound(PendingEntitySound {
            position: entity.position(),
            resource_id: resource_id.to_string(),
            source: SoundSource::Neutral,
            volume: 0.8,
            pitch,
        });
    }
}

// Horse.java:130-132.
pub(in crate::entity) const fn horse_ambient_sound(baby: bool) -> &'static str {
    if baby {
        "minecraft:entity.baby_horse.ambient"
    } else {
        "minecraft:entity.horse.ambient"
    }
}

// SkeletonHorse.java:68-70.
pub(in crate::entity) fn skeleton_horse_ambient_sound(entity: &Entity) -> &'static str {
    if entity.is_eye_in_fluid(crate::entity::environment::FluidTag::Water) {
        "minecraft:entity.skeleton_horse.ambient_water"
    } else {
        "minecraft:entity.skeleton_horse.ambient"
    }
}

fn abstract_horse_ambient_sound(entity: &Entity) -> Option<&'static str> {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    match data.entity_resource_id() {
        "minecraft:camel" => Some("minecraft:entity.camel.ambient"),
        "minecraft:camel_husk" => Some("minecraft:entity.camel_husk.ambient"),
        "minecraft:donkey" => Some("minecraft:entity.donkey.ambient"),
        "minecraft:horse" => Some(horse_ambient_sound(
            data.ageable_retained()
                .is_some_and(AgeableMobRetainedState::is_baby),
        )),
        "minecraft:llama" | "minecraft:trader_llama" => Some("minecraft:entity.llama.ambient"),
        "minecraft:mule" => Some("minecraft:entity.mule.ambient"),
        "minecraft:skeleton_horse" => Some(skeleton_horse_ambient_sound(entity)),
        "minecraft:zombie_horse" => Some("minecraft:entity.zombie_horse.ambient"),
        _ => None,
    }
}

/// `AbstractHorse.canPerformRearing` (`AbstractHorse.java:329-331`) and the two
/// overrides that refuse it (`Llama.java:259-261`, `Camel.java:464-466`).
const fn can_perform_rearing(entity_type_id: i32) -> bool {
    !matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_LLAMA_ID
            | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID
            | VANILLA_ENTITY_TYPE_CAMEL_ID
            | VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
    )
}

fn stand_if_possible(entity: &mut Entity) -> bool {
    if !can_perform_rearing(entity.entity_type_id()) {
        return false;
    }
    match &mut entity.data {
        EntityData::GeneralLiving(data) => data
            .abstract_horse_retained_mut()
            .is_some_and(|horse| horse.set_standing(20)),
        _ => false,
    }
}

/// `AbstractHorse.canJump` and `handleStartJump` (`AbstractHorse.java:892-901`)
/// on the saddled horse a player controls; false when it cannot jump.
pub(crate) fn handle_abstract_horse_start_jump(
    entity: &mut Entity,
    store: &mut EntityStore,
    tags: &crate::entity::EntityRegistryTagView,
) -> bool {
    let jump_sound = match entity.entity_type_id() {
        VANILLA_ENTITY_TYPE_CAMEL_ID | VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID => return false,
        // `Donkey.java:54-56`, `Mule.java:44-46`, `SkeletonHorse.java:112-118`.
        VANILLA_ENTITY_TYPE_DONKEY_ID => "minecraft:entity.donkey.jump",
        VANILLA_ENTITY_TYPE_MULE_ID => "minecraft:entity.mule.jump",
        VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID if entity.is_in_water() => {
            "minecraft:entity.skeleton_horse.jump_water"
        }
        _ => "minecraft:entity.horse.jump",
    };
    let is_horse = matches!(
        entity.data(),
        EntityData::GeneralLiving(data) if data.abstract_horse_retained().is_some()
    );
    if !is_horse
        || crate::entity::living::valid_equipment::has_valid_item_with_registry_tags(
            entity,
            tags,
            EquipmentSlot::Saddle,
        ) != Some(true)
    {
        return false;
    }
    if stand_if_possible(entity) {
        entity.mark_synched_dirty(VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID);
    }
    if !entity.is_silent() {
        store.play_entity_sound(PendingEntitySound {
            position: entity.position(),
            resource_id: jump_sound.to_owned(),
            source: SoundSource::Neutral,
            volume: 0.4,
            pitch: 1.0,
        });
    }
    true
}

pub(crate) fn apply_abstract_horse_make_mad(entity: &mut Entity, store: &mut EntityStore) -> bool {
    let standing = match entity.data() {
        EntityData::GeneralLiving(data) => data
            .abstract_horse_retained()
            .map(AbstractHorseRetainedState::is_standing),
        _ => None,
    };
    let Some(standing) = standing else {
        return true;
    };
    if standing {
        return true;
    }
    if stand_if_possible(entity) {
        entity.mark_synched_dirty(VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID);
    }
    let is_baby = matches!(entity.data(), EntityData::GeneralLiving(data)
        if data.ageable_retained().is_some_and(AgeableMobRetainedState::is_baby));
    let Some(resource_id) = abstract_horse_angry_sound(entity.entity_type_id(), is_baby) else {
        return true;
    };
    let Some(pitch) = store
        .mob_ai_mut(entity.id())
        .map(|runtime| runtime.voice_pitch(is_baby))
    else {
        return true;
    };
    if !entity.is_silent() {
        store.play_entity_sound(PendingEntitySound {
            position: entity.position(),
            resource_id: resource_id.to_string(),
            source: SoundSource::Neutral,
            volume: 0.8,
            pitch,
        });
    }
    true
}

/// `AbstractHorse.getAngrySound`, null by default (`AbstractHorse.java:337-339`)
/// and overridden by five subtypes (`Horse.java:150-152`, `Donkey.java:25-27`,
/// `Mule.java:24-26`, `Llama.java:264-266`, `ZombieHorse.java:105-107`).
const fn abstract_horse_angry_sound(entity_type_id: i32, is_baby: bool) -> Option<&'static str> {
    match entity_type_id {
        VANILLA_ENTITY_TYPE_HORSE_ID => Some(if is_baby {
            "minecraft:entity.baby_horse.angry"
        } else {
            "minecraft:entity.horse.angry"
        }),
        VANILLA_ENTITY_TYPE_DONKEY_ID => Some("minecraft:entity.donkey.angry"),
        VANILLA_ENTITY_TYPE_MULE_ID => Some("minecraft:entity.mule.angry"),
        VANILLA_ENTITY_TYPE_LLAMA_ID | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID => {
            Some("minecraft:entity.llama.angry")
        }
        VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID => Some("minecraft:entity.zombie_horse.angry"),
        _ => None,
    }
}

/// `AbstractHorse.getMaxTemper` (`AbstractHorse.java:392-394`) and `Llama`'s
/// override (`Llama.java:309-311`).
pub(crate) const fn abstract_horse_max_temper(entity_type_id: i32) -> i32 {
    match entity_type_id {
        VANILLA_ENTITY_TYPE_LLAMA_ID | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID => 30,
        _ => 100,
    }
}

pub(crate) fn modify_abstract_horse_temper(entity: &mut Entity, amount: i32) -> Option<i32> {
    let max_temper = abstract_horse_max_temper(entity.entity_type_id());
    match &mut entity.data {
        EntityData::GeneralLiving(data) => data
            .abstract_horse_retained_mut()
            .map(|horse| horse.modify_temper(amount, max_temper)),
        _ => None,
    }
}

pub(crate) fn tame_abstract_horse_with_name(
    entity: &mut Entity,
    store: &mut EntityStore,
    player_uuid: Uuid,
) -> bool {
    let entity_id = entity.id();
    let entity_type_id = entity.entity_type_id();
    let changed = match &mut entity.data {
        EntityData::GeneralLiving(data) => data.abstract_horse_retained_mut().map(|horse| {
            horse.set_owner(Some(player_uuid));
            horse.set_tame(true)
        }),
        _ => None,
    };
    let Some(changed) = changed else {
        return false;
    };
    if changed {
        entity.mark_synched_dirty(VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID);
    }
    store.queue_command(crate::entity::EntityCommand::TamedAnimal(
        crate::entity::store::PendingTamedAnimal {
            player_uuid,
            entity_id,
            entity_type_id,
        },
    ));
    // `this.level().broadcastEntityEvent(this, (byte)7)` (`:716`).
    store.broadcast_to_tracking(
        entity_id,
        ClientboundPlayPacket::EntityEvent(bbb_protocol::packets::EntityEvent {
            entity_id,
            event_id: TAMING_SUCCEEDED_ENTITY_EVENT,
        }),
    );
    true
}

/// `TamableAnimal.handleEntityEvent`'s two bytes, which `AbstractHorse` shares
/// (`TamableAnimal.java:105-113`, `AbstractHorse.java:716`,
/// `RunAroundLikeCrazyGoal.java:71`).
const TAMING_SUCCEEDED_ENTITY_EVENT: i8 = 7;
pub(crate) const TAMING_FAILED_ENTITY_EVENT: i8 = 6;

pub(crate) fn do_player_ride_body_writes(entity: &mut Entity) {
    let changed = match &mut entity.data {
        EntityData::GeneralLiving(data) => data.abstract_horse_retained_mut().map(|horse| {
            let eating = horse.set_eating(false);
            let standing = horse.clear_standing();
            eating || standing
        }),
        _ => None,
    };
    if changed == Some(true) {
        entity.mark_synched_dirty(VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID);
    }
}

pub(crate) fn apply_run_around_like_crazy(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    outcome: RunAroundLikeCrazyOutcome,
) -> bool {
    match outcome {
        // `this.horse.tameWithName(player)` (`:62`), which keeps the rider on.
        RunAroundLikeCrazyOutcome::Tamed { player_entity_id } => {
            let Some(player) = level.leash_holder_player(player_entity_id) else {
                return false;
            };
            tame_abstract_horse_with_name(entity, store, player.uuid)
        }
        RunAroundLikeCrazyOutcome::Thrown { raised_temper } => {
            if raised_temper && modify_abstract_horse_temper(entity, 5).is_none() {
                return false;
            }
            // `this.horse.ejectPassengers()` (`:69`).
            store.eject_body_passengers_reverse(entity);
            // `this.horse.makeMad()` (`:70`).
            if !apply_abstract_horse_make_mad(entity, store) {
                return false;
            }
            // `this.horse.level().broadcastEntityEvent(this.horse, (byte)6)` (`:71`).
            store.broadcast_to_tracking(
                entity.id(),
                ClientboundPlayPacket::EntityEvent(bbb_protocol::packets::EntityEvent {
                    entity_id: entity.id(),
                    event_id: TAMING_FAILED_ENTITY_EVENT,
                }),
            );
            true
        }
    }
}
