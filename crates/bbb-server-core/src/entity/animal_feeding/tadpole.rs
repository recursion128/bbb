//! `Tadpole.mobInteract`'s feeding and golden dandelion arms
//! (`Tadpole.java:154-165`); the bucket arm is `bucketable.rs`'s.

use bbb_protocol::entity_synched_data::VANILLA_TADPOLE_AGE_LOCKED_DATA_ID;
use bbb_protocol::item_types::VANILLA_ITEM_GOLDEN_DANDELION_ID;

use super::age_lock::{play_toggle_sound, CANNOT_BE_AGE_LOCKED_TAG, ENTITY_TYPE_REGISTRY};
use super::{AnimalFeeding, FeedingPlayer, FoodPredicate};
use crate::entity::level::EntityRegistryTagView;
use crate::entity::{EntityCommand, EntityData, EntityStore};

/// `Tadpole.isFood` (`Tadpole.java:208-210`).
const FROG_FOOD: FoodPredicate = FoodPredicate::Tag("minecraft:frog_food");

pub(super) fn mob_interact(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut FeedingPlayer<'_>,
    tags: &EntityRegistryTagView,
) -> Option<AnimalFeeding> {
    let target = store.get_mut(target_id)?;
    let block_position = target.block_position();
    let EntityData::GeneralLiving(data) = &mut target.data else {
        return None;
    };
    let state = data.tadpole_retained_mut()?;
    if FROG_FOOD.matches(player.hand_stack.item_id, tags) && !state.age_locked() {
        let grown = state.feed();
        super::use_player_item(player, false);
        if grown {
            store.queue_command(EntityCommand::TadpoleGrowUp {
                entity_id: target_id,
            });
        }
        return Some(AnimalFeeding::GrewUp);
    }
    // `AgeableMob.canUseGoldenDandelion(itemStack, true, ...)` (`AgeableMob.java:78-80`).
    if player.hand_stack.item_id != Some(VANILLA_ITEM_GOLDEN_DANDELION_ID)
        || state.age_lock_particle_timer() != 0
        || tags.contains(
            ENTITY_TYPE_REGISTRY,
            CANNOT_BE_AGE_LOCKED_TAG,
            "minecraft:tadpole",
        )
    {
        return Some(AnimalFeeding::Pass);
    }
    let locked = state.toggle_age_lock();
    target.mark_synched_dirty(VANILLA_TADPOLE_AGE_LOCKED_DATA_ID);
    if locked {
        crate::entity::conversion::set_persistence_required(target);
    }
    super::use_player_item(player, false);
    play_toggle_sound(store, block_position, locked);
    Some(AnimalFeeding::AgeLockToggled)
}
