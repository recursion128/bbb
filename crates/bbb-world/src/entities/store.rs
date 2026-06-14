use std::{collections::BTreeMap, fmt};

use hecs::{Entity, World};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{EntityIdentity, EntityState, EntityTransform};

pub(crate) struct EntityStore {
    ecs: World,
    by_protocol_id: BTreeMap<i32, Entity>,
    order: Vec<i32>,
    snapshots: Vec<EntityState>,
    snapshot_index: BTreeMap<i32, usize>,
}

impl EntityStore {
    pub(crate) fn insert_or_replace(&mut self, state: EntityState) {
        if let Some(entity) = self.by_protocol_id.get(&state.id).copied() {
            if self.replace_existing_components(entity, state.clone()) {
                self.update_snapshot(state);
                return;
            }
            let _ = self.ecs.despawn(entity);
            self.by_protocol_id.remove(&state.id);
        }

        let id = state.id;
        let entity = self.ecs.spawn((
            EntityIdentity::from(&state),
            EntityTransform::from(&state),
            state.clone(),
        ));
        self.by_protocol_id.insert(id, entity);
        if !self.snapshot_index.contains_key(&id) {
            self.order.push(id);
        }
        self.update_snapshot(state);
    }

    pub(crate) fn get(&self, id: i32) -> Option<&EntityState> {
        self.snapshot_index
            .get(&id)
            .and_then(|index| self.snapshots.get(*index))
    }

    pub(crate) fn contains(&self, id: i32) -> bool {
        self.by_protocol_id.contains_key(&id)
    }

    pub(crate) fn entity_type_id(&self, id: i32) -> Option<i32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityIdentity>(entity)
            .ok()
            .map(|identity| identity.entity_type_id)
    }

    pub(crate) fn transform(&self, id: i32) -> Option<EntityTransform> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityTransform>(entity)
            .ok()
            .map(|transform| *transform)
    }

    pub(crate) fn with_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityState) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut state = self.ecs.get::<&mut EntityState>(entity).ok()?;
        let result = update(&mut state);
        let snapshot = (*state).clone();
        drop(state);
        self.sync_components_from_state(entity, &snapshot);
        self.update_snapshot(snapshot);
        Some(result)
    }

    pub(crate) fn with_transform_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityTransform) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut transform = self.ecs.get::<&mut EntityTransform>(entity).ok()?;
        let result = update(&mut transform);
        let snapshot_transform = *transform;
        drop(transform);
        self.sync_transform_to_state(entity, snapshot_transform);
        Some(result)
    }

    pub(crate) fn for_each_mut(&mut self, mut update: impl FnMut(&mut EntityState)) {
        let ids = self.order.clone();
        for id in ids {
            let _ = self.with_mut(id, |entity| update(entity));
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &EntityState> {
        self.snapshots.iter()
    }

    pub(crate) fn len(&self) -> usize {
        self.by_protocol_id.len()
    }

    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn remove_ids(&mut self, ids: &[i32]) -> usize {
        let mut removed = 0;
        for id in ids {
            let Some(entity) = self.by_protocol_id.remove(id) else {
                continue;
            };
            let _ = self.ecs.despawn(entity);
            removed += 1;
        }
        if removed > 0 {
            self.rebuild_snapshots_from_ecs();
        }
        removed
    }

    fn update_snapshot(&mut self, state: EntityState) {
        if let Some(index) = self.snapshot_index.get(&state.id).copied() {
            self.snapshots[index] = state;
            return;
        }
        let index = self.snapshots.len();
        self.snapshot_index.insert(state.id, index);
        self.snapshots.push(state);
    }

    fn replace_existing_components(&mut self, entity: Entity, state: EntityState) -> bool {
        let replaced_state = {
            if let Ok(mut existing) = self.ecs.get::<&mut EntityState>(entity) {
                *existing = state.clone();
                true
            } else {
                false
            }
        };
        if !replaced_state {
            return false;
        }

        self.sync_components_from_state(entity, &state);
        true
    }

    fn sync_components_from_state(&mut self, entity: Entity, state: &EntityState) {
        if let Ok(mut identity) = self.ecs.get::<&mut EntityIdentity>(entity) {
            *identity = EntityIdentity::from(state);
        }
        if let Ok(mut transform) = self.ecs.get::<&mut EntityTransform>(entity) {
            *transform = EntityTransform::from(state);
        }
    }

    fn sync_transform_to_state(&mut self, entity: Entity, transform: EntityTransform) {
        let Ok(mut state) = self.ecs.get::<&mut EntityState>(entity) else {
            return;
        };
        transform.write_to_state(&mut state);
        let snapshot = (*state).clone();
        drop(state);
        self.update_snapshot(snapshot);
    }

    fn rebuild_snapshots_from_ecs(&mut self) {
        self.order.retain(|id| self.by_protocol_id.contains_key(id));
        self.snapshots.clear();
        self.snapshot_index.clear();
        let order = self.order.clone();
        for id in order {
            let Some(entity) = self.by_protocol_id.get(&id).copied() else {
                continue;
            };
            let Ok(state) = self.ecs.get::<&EntityState>(entity) else {
                continue;
            };
            let snapshot = (*state).clone();
            drop(state);
            self.update_snapshot(snapshot);
        }
    }
}

impl Default for EntityStore {
    fn default() -> Self {
        Self {
            ecs: World::new(),
            by_protocol_id: BTreeMap::new(),
            order: Vec::new(),
            snapshots: Vec::new(),
            snapshot_index: BTreeMap::new(),
        }
    }
}

impl Clone for EntityStore {
    fn clone(&self) -> Self {
        let mut store = Self::default();
        for state in &self.snapshots {
            store.insert_or_replace(state.clone());
        }
        store
    }
}

impl fmt::Debug for EntityStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EntityStore")
            .field("entities", &self.snapshots)
            .finish()
    }
}

impl Serialize for EntityStore {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.snapshots.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EntityStore {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let states = Vec::<EntityState>::deserialize(deserializer)?;
        let mut store = EntityStore::default();
        for state in states {
            store.insert_or_replace(state);
        }
        Ok(store)
    }
}
