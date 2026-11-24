use super::{Entity, EntityLifecycleEvent, EntityStore, SectionKey};
use crate::level_ticks::ChunkKey;
use rustc_hash::FxHashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityVisibility {
    Hidden,
    Tracked,
    Ticking,
}

impl EntityVisibility {
    pub const fn is_accessible(self) -> bool {
        !matches!(self, Self::Hidden)
    }

    pub const fn is_ticking(self) -> bool {
        matches!(self, Self::Ticking)
    }
}

/// Dense O(1) membership set for tracked/ticking entity ids (Java tick insertion
/// order is a non-goal, so no BTreeMap or Arc copy-on-write overhead is kept).
#[derive(Clone, Debug, Default)]
struct EntityIdSet {
    ids: Vec<i32>,
    index_by_id: FxHashMap<i32, usize>,
}

impl EntityIdSet {
    fn insert(&mut self, id: i32) {
        if self.index_by_id.contains_key(&id) {
            return;
        }
        let index = self.ids.len();
        self.ids.push(id);
        self.index_by_id.insert(id, index);
    }

    fn remove(&mut self, id: &i32) {
        let Some(index) = self.index_by_id.remove(id) else {
            return;
        };
        self.ids.swap_remove(index);
        if let Some(&moved) = self.ids.get(index) {
            self.index_by_id.insert(moved, index);
        }
    }

    fn contains(&self, id: i32) -> bool {
        self.index_by_id.contains_key(&id)
    }

    fn iter(&self) -> std::slice::Iter<'_, i32> {
        self.ids.iter()
    }

    fn clone_vec(&self) -> Vec<i32> {
        self.ids.clone()
    }
}

#[derive(Clone, Debug)]
pub(super) struct EntityVisibilityState {
    pub(super) default: EntityVisibility,
    chunks: Arc<FxHashMap<ChunkKey, EntityVisibility>>,
    visible: EntityIdSet,
    ticking: EntityIdSet,
}

impl Default for EntityVisibilityState {
    fn default() -> Self {
        Self {
            default: EntityVisibility::Ticking,
            chunks: Arc::default(),
            visible: EntityIdSet::default(),
            ticking: EntityIdSet::default(),
        }
    }
}

impl EntityVisibilityState {
    pub(super) fn forget(&mut self, id: i32) {
        self.visible.remove(&id);
        self.ticking.remove(&id);
    }

    fn transition_tracking(
        &mut self,
        lifecycle_events: &mut Vec<EntityLifecycleEvent>,
        entity: &Entity,
        accessible: bool,
    ) {
        if accessible {
            self.visible.insert(entity.id());
            lifecycle_events.push(EntityLifecycleEvent::TrackingStarted {
                entity: entity.clone(),
            });
        } else {
            lifecycle_events.push(EntityLifecycleEvent::TrackingStopped {
                entity: entity.clone(),
            });
            self.visible.remove(&entity.id());
        }
    }

    // PersistentEntitySectionManager.java:396-423: movement changes tracking before tick membership.
    pub(super) fn transition_section_visibility(
        &mut self,
        lifecycle_events: &mut Vec<EntityLifecycleEvent>,
        entity: &Entity,
        from: SectionKey,
        to: SectionKey,
        previous: EntityVisibility,
        current: EntityVisibility,
    ) {
        if previous.is_accessible() != current.is_accessible() {
            self.transition_tracking(lifecycle_events, entity, current.is_accessible());
        }
        if previous.is_ticking() && !current.is_ticking() {
            self.ticking.remove(&entity.id());
        } else if !previous.is_ticking() && current.is_ticking() {
            self.ticking.insert(entity.id());
        }
        if current.is_accessible() {
            lifecycle_events.push(EntityLifecycleEvent::Moved {
                entity_id: entity.id(),
                from,
                to,
            });
        }
    }
}

impl EntityStore {
    pub fn new_managed(random: crate::random::GameRandom, ids: super::EntityIdCounter) -> Self {
        let mut store = Self::with_random(random, ids);
        store.visibility.default = EntityVisibility::Hidden;
        store
    }

    pub fn section_visibility(&self, section: SectionKey) -> EntityVisibility {
        self.visibility
            .chunks
            .get(&section.column())
            .copied()
            .unwrap_or(self.visibility.default)
    }

    pub fn get_visible(&self, id: i32) -> Option<&Entity> {
        self.visibility
            .visible
            .contains(id)
            .then(|| self.get(id))
            .flatten()
    }

    pub fn visible_entity_id_for_uuid(&self, uuid: Uuid) -> Option<i32> {
        let id = self.entity_id_for_uuid(uuid)?;
        self.visibility.visible.contains(id).then_some(id)
    }

    pub fn visible_ids(&self) -> impl Iterator<Item = i32> + '_ {
        self.visibility.visible.iter().copied()
    }

    pub fn ticking_ids(&self) -> Vec<i32> {
        self.visibility.ticking.clone_vec()
    }

    pub fn is_ticking(&self, id: i32) -> bool {
        self.visibility.ticking.contains(id)
    }

    // PersistentEntitySectionManager.java:178-226; Entity.java:3876-3882, Entity.java:3463-3465.
    pub fn unload_chunk_entities_after_save(&mut self, chunk: ChunkKey) {
        let roots = self
            .ids_in_chunk_section_order(chunk)
            .into_iter()
            .filter(|id| {
                self.get(*id).is_some_and(|entity| {
                    entity
                        .removal_reason()
                        .is_none_or(|reason| reason.should_save())
                }) && self.vehicle(*id).is_none()
                    && self.external_player_count_in_tree(*id) != 1
            })
            .collect::<Vec<_>>();
        for root in roots {
            if let Some(ids) = self.entity_tree_ids(root) {
                for id in ids {
                    self.remove(id, crate::entity::RemovalReason::UnloadedToChunk);
                }
            }
        }
        if !self.is_chunk_occupied(chunk) {
            self.chunk_dirty_epochs.remove(&chunk);
        }
    }

    // PersistentEntitySectionManager.java:133-169: each section stops ticks before tracking changes.
    pub fn apply_chunk_visibility(&mut self, chunk: ChunkKey, visibility: EntityVisibility) {
        let previous = self.section_visibility(SectionKey::new(chunk.0, 0, chunk.1));
        if previous == visibility {
            return;
        }
        if visibility == self.visibility.default {
            Arc::make_mut(&mut self.visibility.chunks).remove(&chunk);
        } else {
            Arc::make_mut(&mut self.visibility.chunks).insert(chunk, visibility);
        }
        let sections = self
            .sections_by_column
            .get(&chunk)
            .cloned()
            .unwrap_or_default();
        for section in sections {
            let ids = self.by_section[&section]
                .iter()
                .copied()
                .filter(|id| self.player(*id).is_none())
                .collect::<Vec<_>>();
            if previous.is_ticking() && !visibility.is_ticking() {
                for id in &ids {
                    self.visibility.ticking.remove(id);
                }
            }
            if previous.is_accessible() != visibility.is_accessible() {
                for id in &ids {
                    if let Some(entity) = self.entities.get(id).and_then(|slot| slot.body.as_ref())
                    {
                        self.visibility.transition_tracking(
                            &mut self.lifecycle_events,
                            entity,
                            visibility.is_accessible(),
                        );
                        if visibility.is_accessible() {
                            self.publish_existing_passengers(*id);
                        }
                    }
                }
            }
            if !previous.is_ticking() && visibility.is_ticking() {
                for id in ids {
                    self.visibility.ticking.insert(id);
                }
            }
        }
    }

    // PersistentEntitySectionManager.java:75-96: one admission owns identity, tracking, then ticks.
    pub(super) fn admit_entity(&mut self, mut entity: Box<Entity>) {
        let id = entity.id();
        if crate::entity::ender_dragon::state(&entity).is_some() {
            self.dragon_ids.insert(id);
        }
        let section = super::section_of_block(entity.block_position());
        entity.set_filed_section(section);
        self.file_in_section(id, section);
        let visibility = self.section_visibility(section);
        let tracked = visibility.is_accessible();
        if tracked {
            self.visibility.visible.insert(id);
        }
        self.lifecycle_events.push(EntityLifecycleEvent::Added {
            entity: (*entity).clone(),
            tracked,
        });
        if visibility.is_ticking() {
            self.visibility.ticking.insert(id);
        }
        self.index_body_metadata(&entity);
        if self
            .entities
            .entry(id)
            .or_default()
            .body
            .replace(entity)
            .is_none()
        {
            self.live_body_count += 1;
        }
        self.mark_entity_chunk_dirty(id);
    }

    pub(super) fn transition_section_visibility(
        &mut self,
        entity: &Entity,
        from: SectionKey,
        to: SectionKey,
    ) {
        let previous = self.section_visibility(from);
        let current = self.section_visibility(to);
        self.visibility.transition_section_visibility(
            &mut self.lifecycle_events,
            entity,
            from,
            to,
            previous,
            current,
        );
        if !previous.is_accessible() && current.is_accessible() {
            self.publish_existing_passengers(entity.id());
        }
    }

    // ServerEntity.java:303-305: newly tracked vehicles publish existing passengers.
    pub(super) fn publish_existing_passengers(&mut self, id: i32) {
        let count = self.passengers(id).len();
        if count != 0 {
            self.publish_passengers_changed(id, count, None);
        }
    }

    // PersistentEntitySectionManager.java:427-448: stopping membership precedes actual identity removal.
    pub(super) fn publish_removed(&mut self, entity: Entity) {
        let tracked = self.visibility.visible.contains(entity.id());
        self.visibility.forget(entity.id());
        self.lifecycle_events
            .push(EntityLifecycleEvent::Removed { entity, tracked });
    }
}

#[cfg(test)]
mod tests;
