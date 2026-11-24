use super::{Entity, EntityData, EntityStore};
use crate::world::simulation_level::LevelCtx;

impl EntityStore {
    pub(crate) fn publish_detached_hanging_removal(&mut self, entity: &Entity) -> Option<()> {
        if entity.removal_reason().is_none()
            || !matches!(
                entity.data(),
                EntityData::Painting(_)
                    | EntityData::ItemFrame(_)
                    | EntityData::GlowItemFrame(_)
                    | EntityData::LeashKnot(_)
            )
        {
            return None;
        }
        self.publish_detached_removal_edge(entity)
    }

    fn publish_detached_removal_edge(&mut self, entity: &Entity) -> Option<()> {
        if entity.removal_reason().is_none()
            || !self.is_detached(entity.id())
            || self.ids_by_uuid.get(&entity.uuid()) != Some(&entity.id())
        {
            return None;
        }
        let slot = self.entities.get_mut(&entity.id())?;
        if !std::mem::replace(&mut slot.removal_published, true) {
            self.mark_chunk_dirty(entity.filed_section().column());
            // Drops follow removal, but lavaHurt still needs the body's RNG afterward.
            // Final attach owns RNG/identity cleanup and skips this already queued edge.
            self.unlink_entity(entity.id(), None);
            self.publish_removed(entity.clone());
        }
        Some(())
    }
    pub(crate) fn publish_detached_vehicle_removal(
        &mut self,
        entity: &mut Entity,
        level: &mut LevelCtx<'_, '_>,
    ) -> Option<()> {
        if (!crate::entity::boat::is_boat(entity.data())
            && !crate::entity::minecart::is_minecart(entity.data()))
            || entity.removal_reason().is_none()
            || !self.is_detached(entity.id())
            || self.ids_by_uuid.get(&entity.uuid()) != Some(&entity.id())
        {
            return None;
        }
        if !self.removal_published(entity.id()) && entity.removal_reason()?.should_destroy() {
            level.unpack_vehicle_container_body(
                entity,
                crate::entity::level::EntityBlockContext::new(Some(self), None),
            )?;
            self.drop_destroyed_container_cargo(entity).then_some(())?;
            crate::entity::leash::drop_leash_on_destroy(entity, self);
        }
        self.publish_detached_removal_edge(entity)
    }

    pub(crate) fn finish_detached_vehicle_destroy(&mut self, entity: &mut Entity) -> Option<()> {
        if !self.removal_published(entity.id()) {
            return None;
        }
        self.drop_destroyed_container_cargo(entity).then_some(())
    }

    fn removal_published(&self, id: i32) -> bool {
        self.entities
            .get(&id)
            .is_some_and(|slot| slot.removal_published)
    }
}
