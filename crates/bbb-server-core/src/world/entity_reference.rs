//! `EntityReference` (`net/minecraft/world/entity/EntityReference.java`): how
//! one remembered UUID becomes an entity again.
//!
//! Both arms of `getEntity(Level, Class)` (`EntityReference.java:78-82`) look
//! past the level holding the reference, so a remembered attacker that walked
//! through a portal is still the same entity. The two readers of that rule in
//! this world are the kill credit `LivingEntity.die` takes
//! (`LivingEntity.java:1946-1952`) and the credit a stored death queues.

use super::*;
use crate::world::LevelId;

impl ServerWorld {
    /// The `Player.class` arm, whose lookup is
    /// `ServerLevel.getPlayerInAnyDimension` -- the server's own player list
    /// (`ServerLevel.java:1397-1399`), so the holder's level decides nothing
    /// and only an offline UUID answers `null`.
    pub(in crate::world) fn player_reference(&self, uuid: Uuid) -> Option<ConnectionId> {
        self.connection_of(uuid)
    }

    /// The other arm, over `ServerLevel.getEntityInAnyDimension`
    /// (`ServerLevel.java:1378-1394`): the holder's own level answers first and
    /// every other level is walked after it. Vanilla walks `getAllLevels()` in
    /// the server's order and this walks the level map's; a UUID is filed in
    /// one store at a time, so the order decides nothing.
    pub(in crate::world) fn entity_reference(
        &self,
        holder: &LevelId,
        uuid: Uuid,
    ) -> Option<(LevelId, i32)> {
        if let Some(entity_id) = self
            .levels
            .get(holder)
            .and_then(|level| level.entities.entity_id_for_uuid(uuid))
        {
            return Some((*holder, entity_id));
        }
        self.levels
            .iter()
            .filter(|(key, _)| *key != *holder)
            .find_map(|(key, level)| {
                level
                    .entities
                    .entity_id_for_uuid(uuid)
                    .map(|entity_id| (key, entity_id))
            })
    }
}
