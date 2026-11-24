//! The shared golden dandelion interaction (`AgeableMob.java:68-102`).

use bbb_protocol::entity_synched_data::VANILLA_AGEABLE_MOB_AGE_LOCKED_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SNIFFER_ID;
use bbb_protocol::item_types::VANILLA_ITEM_GOLDEN_DANDELION_ID;
use bbb_protocol::packets::SoundSource;

use crate::entity::level::EntityRegistryTagView;
use crate::entity::store::{EntityStore, PendingEntitySound};
use crate::entity::{Entity, EntityData};

use super::{AnimalFeeding, FeedingPlayer};

pub(super) const ENTITY_TYPE_REGISTRY: &str = "minecraft:entity_type";

/// `EntityTypeTags.CANNOT_BE_AGE_LOCKED`, the last term of
/// `AgeableMob.canUseGoldenDandelion` (`AgeableMob.java:78-80`).
pub(super) const CANNOT_BE_AGE_LOCKED_TAG: &str = "minecraft:cannot_be_age_locked";

/// `SoundEvents.GOLDEN_DANDELION_USE` (`SoundEvents.java:746`) and
/// `SoundEvents.GOLDEN_DANDELION_UNUSE` (`:747`).
const GOLDEN_DANDELION_USE_SOUND: &str = "minecraft:item.golden_dandelion.use";
const GOLDEN_DANDELION_UNUSE_SOUND: &str = "minecraft:item.golden_dandelion.unuse";

/// `AgeableMob.getBabyStartAge` (`AgeableMob.java:236-238`) and its one
/// override (`Sniffer.java:117-119`).
const fn baby_start_age(entity_type_id: i32) -> i32 {
    match entity_type_id {
        VANILLA_ENTITY_TYPE_SNIFFER_ID => -48_000,
        _ => -24_000,
    }
}

pub(super) fn mob_interact(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut FeedingPlayer<'_>,
    tags: &EntityRegistryTagView,
) -> AnimalFeeding {
    if player.hand_stack.item_id != Some(VANILLA_ITEM_GOLDEN_DANDELION_ID) {
        return AnimalFeeding::Pass;
    }
    let Some(target) = store.get(target_id) else {
        return AnimalFeeding::Pass;
    };
    let EntityData::GeneralLiving(data) = target.data() else {
        return AnimalFeeding::Pass;
    };
    let entity_resource_id = data.entity_resource_id();
    let Some(ageable) = data.ageable_retained() else {
        return AnimalFeeding::Pass;
    };
    // `canUseGoldenDandelion` (`AgeableMob.java:78-80`).
    if !data.is_baby() || ageable.age_lock_particle_timer() != 0 {
        return AnimalFeeding::Pass;
    }
    if tags.contains(
        ENTITY_TYPE_REGISTRY,
        CANNOT_BE_AGE_LOCKED_TAG,
        entity_resource_id,
    ) {
        return AnimalFeeding::Pass;
    }
    let baby_start_age = baby_start_age(target.entity_type_id());
    let block_position = target.block_position();
    let Some(mut target) = store.detach(target_id) else {
        return AnimalFeeding::Pass;
    };
    // `setAgeLocked` runs `setAgeLockedData` before the consume
    // (`AgeableMob.java:88-96`, `:82-86`).
    let Some(locked) = toggle_age_lock(&mut target, baby_start_age) else {
        store.attach(target);
        return AnimalFeeding::Pass;
    };
    if locked {
        // `mob.setPersistenceRequired()` (`:94-96`).
        set_persistence_required(&mut target);
    }
    store.attach(target);
    // `itemInHand.consume(1, player)` (`:92`).
    super::use_player_item(player, false);
    play_toggle_sound(store, block_position, locked);
    AnimalFeeding::AgeLockToggled
}

/// `mob.level().playSound(null, mob.blockPosition(), ...)` (`:98-101`) over
/// the block centre `Level.playSound` takes (`Level.java:389-392`).
pub(super) fn play_toggle_sound(
    store: &mut EntityStore,
    block_position: bbb_protocol::packets::BlockPos,
    locked: bool,
) {
    store.play_entity_sound(PendingEntitySound {
        position: bbb_protocol::packets::Vec3d {
            x: f64::from(block_position.x) + 0.5,
            y: f64::from(block_position.y) + 0.5,
            z: f64::from(block_position.z) + 0.5,
        },
        resource_id: if locked {
            GOLDEN_DANDELION_USE_SOUND.to_owned()
        } else {
            GOLDEN_DANDELION_UNUSE_SOUND.to_owned()
        },
        source: SoundSource::Players,
        volume: 1.0,
        pitch: 1.0,
    });
}

/// `AgeableMob.setAgeLockedData` (`AgeableMob.java:82-86`); the answer is the
/// new `AgeLocked`.
fn toggle_age_lock(entity: &mut Entity, baby_start_age: i32) -> Option<bool> {
    let (locked, baby_changed) = match &mut entity.data {
        EntityData::GeneralLiving(data) => {
            let ageable = data.ageable_retained_mut()?;
            let baby_changed = ageable.toggle_age_lock(baby_start_age);
            (ageable.age_locked(), baby_changed)
        }
        _ => return None,
    };
    entity.mark_synched_dirty(VANILLA_AGEABLE_MOB_AGE_LOCKED_DATA_ID);
    if baby_changed {
        crate::entity::general_living::publish_baby_change(entity);
    }
    Some(locked)
}

fn set_persistence_required(entity: &mut Entity) {
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        if let Some(mob) = data.mob.as_mut() {
            mob.set_persistence_required(true);
        }
    }
}
