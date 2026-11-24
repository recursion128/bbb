//! `CriteriaTriggers.TAME_ANIMAL` and `BRED_ANIMALS` as this world fires them.
//!
//! Both call sites that reach it -- `TamableAnimal.tame`
//! (`TamableAnimal.java:171-173`) and `AbstractHorse.tameWithName`
//! (`AbstractHorse.java:712-714`) -- run inside the frame that lends a level
//! its store, which is not where a session's packets are written, so the store
//! records the taming and this drains it.

mod death_message;
pub(in crate::world) use death_message::{PetDamageContext, PetDamageSnapshots};

use super::*;
use crate::world::LevelId;

impl ServerWorld {
    /// Every taming the level's store recorded since the last drain.
    ///
    /// Vanilla holds the `Player` object and fires the trigger behind
    /// `instanceof ServerPlayer` (`TamableAnimal.java:171`); the record carries
    /// the UUID instead, so a UUID the player list does not answer fires
    /// nothing at all. The animal is read back out of the store the record came
    /// from, because `TameAnimalTrigger.TriggerInstance` reads it through an
    /// `EntityPredicate` rather than by type (`TameAnimalTrigger.java:20-22`).
    pub(in crate::world) fn apply_tamed_animal(
        &mut self,
        dimension: &LevelId,
        record: crate::entity::PendingTamedAnimal,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(connection_id) = self.player_reference(record.player_uuid) else {
            return;
        };
        self.trigger_tame_animal(connection_id, *dimension, record.entity_id, events);
    }

    /// `Animal.finalizeSpawnChildFromBreeding`'s love-cause half
    /// (`Animal.java:220-223`).
    pub(in crate::world) fn apply_bred_animals(
        &mut self,
        dimension: &LevelId,
        record: crate::entity::PendingBredAnimals,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(connection_id) = self.player_reference(record.player_uuid) else {
            return;
        };
        self.levels[connection_id].award_stat(
            &mut self.scoreboard,
            bbb_server_types::stats::custom_stat("minecraft:animals_bred"),
            1,
        );
        let parent = self.killed_entity_of_stored(dimension, record.parent_id);
        let partner = self.killed_entity_of_stored(dimension, record.partner_id);
        let child = record
            .child_id
            .map(|child_id| self.killed_entity_of_stored(dimension, child_id));
        self.trigger_bred_animals(connection_id, parent, partner, child, events);
    }
}
