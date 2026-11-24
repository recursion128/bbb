//! Ordered lifecycle publication.

use super::*;

impl EntityStore {
    pub(crate) fn removed_entity(&self, id: i32, uuid: uuid::Uuid) -> Option<&Entity> {
        self.lifecycle_events.iter().find_map(|event| match event {
            EntityLifecycleEvent::Removed { entity, .. }
                if entity.id() == id && entity.uuid() == uuid =>
            {
                Some(entity)
            }
            _ => None,
        })
    }

    #[cfg(test)]
    pub(crate) fn peek_lifecycle_events(&self) -> &[EntityLifecycleEvent] {
        &self.lifecycle_events
    }

    pub fn lifecycle_checkpoint(&self) -> EntityLifecycleCheckpoint {
        EntityLifecycleCheckpoint(self.lifecycle_events.len())
    }

    pub fn take_damage_broadcasts_since(
        &mut self,
        checkpoint: EntityLifecycleCheckpoint,
    ) -> Vec<(i32, bbb_protocol::packets::DamageEvent)> {
        let mut damage = Vec::new();
        let mut index = 0;
        let mut removed_before_sync = 0;
        self.lifecycle_events.retain(|event| {
            let take = if index >= checkpoint.0 {
                if let EntityLifecycleEvent::Broadcast {
                    entity_id,
                    packet: ClientboundPlayPacket::DamageEvent(packet),
                } = event
                {
                    damage.push((*entity_id, *packet));
                    true
                } else {
                    false
                }
            } else {
                false
            };
            if take && index < self.listener_sync_cursor {
                removed_before_sync += 1;
            }
            index += 1;
            !take
        });
        self.listener_sync_cursor -= removed_before_sync;
        damage
    }

    pub(super) fn retain_lifecycle_events(
        &mut self,
        mut keep: impl FnMut(&EntityLifecycleEvent) -> bool,
    ) {
        let mut index = 0;
        let mut removed_before_sync = 0;
        self.lifecycle_events.retain(|event| {
            let kept = keep(event);
            if !kept && index < self.listener_sync_cursor {
                removed_before_sync += 1;
            }
            index += 1;
            kept
        });
        self.listener_sync_cursor -= removed_before_sync;
    }

    pub fn drain_lifecycle_events(&mut self) -> Vec<EntityLifecycleEvent> {
        self.listener_sync_cursor = 0;
        std::mem::take(&mut self.lifecycle_events)
    }

    /// The edges since the game-event listener registry last synced, which it
    /// applies ahead of the tracker's drain so a listener follows its entity
    /// before the next post (`ServerLevel.EntityCallbacks`,
    /// `ServerLevel.java:1942`, `:1970`, `:1975`). A checkpoint rollback that
    /// truncated the journal behind the cursor clamps it.
    pub fn lifecycle_events_since_listener_sync(&self) -> &[EntityLifecycleEvent] {
        &self.lifecycle_events[self.listener_sync_cursor.min(self.lifecycle_events.len())..]
    }

    /// Marks `applied` more edges of [`Self::lifecycle_events_since_listener_sync`]
    /// as applied.
    pub fn advance_listener_sync(&mut self, applied: usize) {
        self.listener_sync_cursor =
            self.listener_sync_cursor.min(self.lifecycle_events.len()) + applied;
    }
}
