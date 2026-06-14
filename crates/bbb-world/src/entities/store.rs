use std::{collections::BTreeMap, fmt};

use hecs::{Entity, World};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{
    EntityAttributes, EntityDamage, EntityEquipment, EntityIdentity, EntityLeash, EntityMetadata,
    EntityMobEffects, EntityMount, EntityState, EntityTransform, EntityTransformState,
    EntityTransientEvents,
};

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
            EntityMetadata::from(&state),
            EntityEquipment::from(&state),
            EntityAttributes::from(&state),
            EntityTransientEvents::from(&state),
            EntityMount::from(&state),
            EntityLeash::from(&state),
            EntityMobEffects::from(&state),
            EntityDamage::from(&state),
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

    pub(crate) fn transform_state(&self, id: i32) -> Option<EntityTransformState> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.transform_state_for_entity(entity)
    }

    pub(crate) fn mount(&self, id: i32) -> Option<EntityMount> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityMount>(entity)
            .ok()
            .map(|mount| (*mount).clone())
    }

    #[cfg(test)]
    pub(crate) fn leash(&self, id: i32) -> Option<EntityLeash> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityLeash>(entity)
            .ok()
            .map(|leash| *leash)
    }

    #[cfg(test)]
    pub(crate) fn mob_effects(&self, id: i32) -> Option<EntityMobEffects> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityMobEffects>(entity)
            .ok()
            .map(|effects| (*effects).clone())
    }

    #[cfg(test)]
    pub(crate) fn damage(&self, id: i32) -> Option<EntityDamage> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityDamage>(entity)
            .ok()
            .map(|damage| *damage)
    }

    pub(crate) fn transform_states(&self) -> Vec<EntityTransformState> {
        let mut transforms = Vec::with_capacity(self.by_protocol_id.len());
        for id in &self.order {
            let Some(entity) = self.by_protocol_id.get(id).copied() else {
                continue;
            };
            if let Some(transform) = self.transform_state_for_entity(entity) {
                transforms.push(transform);
            }
        }
        transforms
    }

    #[cfg(test)]
    pub(crate) fn metadata(&self, id: i32) -> Option<EntityMetadata> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityMetadata>(entity)
            .ok()
            .map(|metadata| (*metadata).clone())
    }

    #[cfg(test)]
    pub(crate) fn equipment(&self, id: i32) -> Option<EntityEquipment> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityEquipment>(entity)
            .ok()
            .map(|equipment| (*equipment).clone())
    }

    #[cfg(test)]
    pub(crate) fn attributes(&self, id: i32) -> Option<EntityAttributes> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityAttributes>(entity)
            .ok()
            .map(|attributes| (*attributes).clone())
    }

    #[cfg(test)]
    pub(crate) fn transient_events(&self, id: i32) -> Option<EntityTransientEvents> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityTransientEvents>(entity)
            .ok()
            .map(|events| *events)
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

    pub(crate) fn with_metadata_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityMetadata) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut metadata = self.ecs.get::<&mut EntityMetadata>(entity).ok()?;
        let result = update(&mut metadata);
        let snapshot_metadata = (*metadata).clone();
        drop(metadata);
        self.sync_metadata_to_state(entity, snapshot_metadata);
        Some(result)
    }

    pub(crate) fn with_equipment_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityEquipment) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut equipment = self.ecs.get::<&mut EntityEquipment>(entity).ok()?;
        let result = update(&mut equipment);
        let snapshot_equipment = (*equipment).clone();
        drop(equipment);
        self.sync_equipment_to_state(entity, snapshot_equipment);
        Some(result)
    }

    pub(crate) fn with_attributes_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityAttributes) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut attributes = self.ecs.get::<&mut EntityAttributes>(entity).ok()?;
        let result = update(&mut attributes);
        let snapshot_attributes = (*attributes).clone();
        drop(attributes);
        self.sync_attributes_to_state(entity, snapshot_attributes);
        Some(result)
    }

    pub(crate) fn with_transient_events_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityTransientEvents) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut events = self.ecs.get::<&mut EntityTransientEvents>(entity).ok()?;
        let result = update(&mut events);
        let snapshot_events = *events;
        drop(events);
        self.sync_transient_events_to_state(entity, snapshot_events);
        Some(result)
    }

    pub(crate) fn with_mount_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityMount) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut mount = self.ecs.get::<&mut EntityMount>(entity).ok()?;
        let result = update(&mut mount);
        let snapshot_mount = (*mount).clone();
        drop(mount);
        self.sync_mount_to_state(entity, snapshot_mount);
        Some(result)
    }

    pub(crate) fn with_leash_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityLeash) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut leash = self.ecs.get::<&mut EntityLeash>(entity).ok()?;
        let result = update(&mut leash);
        let snapshot_leash = *leash;
        drop(leash);
        self.sync_leash_to_state(entity, snapshot_leash);
        Some(result)
    }

    pub(crate) fn with_mob_effects_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityMobEffects) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut effects = self.ecs.get::<&mut EntityMobEffects>(entity).ok()?;
        let result = update(&mut effects);
        let snapshot_effects = (*effects).clone();
        drop(effects);
        self.sync_mob_effects_to_state(entity, snapshot_effects);
        Some(result)
    }

    pub(crate) fn with_damage_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityDamage) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut damage = self.ecs.get::<&mut EntityDamage>(entity).ok()?;
        let result = update(&mut damage);
        let snapshot_damage = *damage;
        drop(damage);
        self.sync_damage_to_state(entity, snapshot_damage);
        Some(result)
    }

    pub(crate) fn for_each_mount_mut(&mut self, mut update: impl FnMut(i32, &mut EntityMount)) {
        let ids = self.order.clone();
        for id in ids {
            let _ = self.with_mount_mut(id, |mount| update(id, mount));
        }
    }

    pub(crate) fn for_each_leash_mut(&mut self, mut update: impl FnMut(i32, &mut EntityLeash)) {
        let ids = self.order.clone();
        for id in ids {
            let _ = self.with_leash_mut(id, |leash| update(id, leash));
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &EntityState> {
        self.snapshots.iter()
    }

    pub(crate) fn total_mob_effects(&self) -> usize {
        self.order
            .iter()
            .filter_map(|id| self.by_protocol_id.get(id).copied())
            .filter_map(|entity| self.ecs.get::<&EntityMobEffects>(entity).ok())
            .map(|effects| effects.effects.len())
            .sum()
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

    fn transform_state_for_entity(&self, entity: Entity) -> Option<EntityTransformState> {
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let transform = self.ecs.get::<&EntityTransform>(entity).ok()?;
        Some(EntityTransformState::from_components(&identity, *transform))
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
        if let Ok(mut metadata) = self.ecs.get::<&mut EntityMetadata>(entity) {
            *metadata = EntityMetadata::from(state);
        }
        if let Ok(mut equipment) = self.ecs.get::<&mut EntityEquipment>(entity) {
            *equipment = EntityEquipment::from(state);
        }
        if let Ok(mut attributes) = self.ecs.get::<&mut EntityAttributes>(entity) {
            *attributes = EntityAttributes::from(state);
        }
        if let Ok(mut events) = self.ecs.get::<&mut EntityTransientEvents>(entity) {
            *events = EntityTransientEvents::from(state);
        }
        if let Ok(mut mount) = self.ecs.get::<&mut EntityMount>(entity) {
            *mount = EntityMount::from(state);
        }
        if let Ok(mut leash) = self.ecs.get::<&mut EntityLeash>(entity) {
            *leash = EntityLeash::from(state);
        }
        if let Ok(mut effects) = self.ecs.get::<&mut EntityMobEffects>(entity) {
            *effects = EntityMobEffects::from(state);
        }
        if let Ok(mut damage) = self.ecs.get::<&mut EntityDamage>(entity) {
            *damage = EntityDamage::from(state);
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

    fn sync_metadata_to_state(&mut self, entity: Entity, metadata: EntityMetadata) {
        let Ok(mut state) = self.ecs.get::<&mut EntityState>(entity) else {
            return;
        };
        metadata.write_to_state(&mut state);
        let snapshot = (*state).clone();
        drop(state);
        self.update_snapshot(snapshot);
    }

    fn sync_equipment_to_state(&mut self, entity: Entity, equipment: EntityEquipment) {
        let Ok(mut state) = self.ecs.get::<&mut EntityState>(entity) else {
            return;
        };
        equipment.write_to_state(&mut state);
        let snapshot = (*state).clone();
        drop(state);
        self.update_snapshot(snapshot);
    }

    fn sync_attributes_to_state(&mut self, entity: Entity, attributes: EntityAttributes) {
        let Ok(mut state) = self.ecs.get::<&mut EntityState>(entity) else {
            return;
        };
        attributes.write_to_state(&mut state);
        let snapshot = (*state).clone();
        drop(state);
        self.update_snapshot(snapshot);
    }

    fn sync_transient_events_to_state(&mut self, entity: Entity, events: EntityTransientEvents) {
        let Ok(mut state) = self.ecs.get::<&mut EntityState>(entity) else {
            return;
        };
        events.write_to_state(&mut state);
        let snapshot = (*state).clone();
        drop(state);
        self.update_snapshot(snapshot);
    }

    fn sync_mount_to_state(&mut self, entity: Entity, mount: EntityMount) {
        let Ok(mut state) = self.ecs.get::<&mut EntityState>(entity) else {
            return;
        };
        mount.write_to_state(&mut state);
        let snapshot = (*state).clone();
        drop(state);
        self.update_snapshot(snapshot);
    }

    fn sync_leash_to_state(&mut self, entity: Entity, leash: EntityLeash) {
        let Ok(mut state) = self.ecs.get::<&mut EntityState>(entity) else {
            return;
        };
        leash.write_to_state(&mut state);
        let snapshot = (*state).clone();
        drop(state);
        self.update_snapshot(snapshot);
    }

    fn sync_mob_effects_to_state(&mut self, entity: Entity, effects: EntityMobEffects) {
        let Ok(mut state) = self.ecs.get::<&mut EntityState>(entity) else {
            return;
        };
        effects.write_to_state(&mut state);
        let snapshot = (*state).clone();
        drop(state);
        self.update_snapshot(snapshot);
    }

    fn sync_damage_to_state(&mut self, entity: Entity, damage: EntityDamage) {
        let Ok(mut state) = self.ecs.get::<&mut EntityState>(entity) else {
            return;
        };
        damage.write_to_state(&mut state);
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
