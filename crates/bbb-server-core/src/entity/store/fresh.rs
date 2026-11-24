//! Entities between `EntityType.create` and `ServerLevel.addFreshEntity`.
//!
//! A fresh body is created in the store, then lifted out with its admission
//! edges dropped, so nothing in the level observes it while it is finalized.
//! Its passengers live in the store already, linked to its id.

use super::*;

impl EntityStore {
    /// Lifts a just-created entity back out of the level before it is added.
    pub(crate) fn take_fresh(&mut self, id: i32) -> Option<Box<Entity>> {
        let entity = self.detach(id)?;
        self.unfile(id, entity.filed_section());
        self.visibility.forget(id);
        self.retain_lifecycle_events(|event| {
            !matches!(
                event,
                EntityLifecycleEvent::Added { entity, .. } | EntityLifecycleEvent::TrackingStarted { entity }
                    if entity.id() == id
            )
        });
        Some(entity)
    }

    /// `ServerLevel.addFreshEntityWithPassengers` for a fresh body whose
    /// passengers are already in the store.
    pub fn add_fresh(&mut self, entity: Box<Entity>) -> i32 {
        let id = entity.id();
        let school_leader =
            crate::entity::schooling_fish_state(&entity).and_then(|state| state.leader_uuid());
        let vehicle = self.vehicle(id);
        // Relation edges named before the body was tracked are re-sent after it.
        self.retain_lifecycle_events(|event| {
            !matches!(
                event,
                EntityLifecycleEvent::PassengersChanged { vehicle_id, .. }
                    if *vehicle_id == id || Some(*vehicle_id) == vehicle
            )
        });
        self.admit_entity(entity);
        for vehicle_id in vehicle.into_iter().chain([id]) {
            let passenger_ids = self.passengers(vehicle_id).to_vec();
            if !passenger_ids.is_empty() {
                self.lifecycle_events
                    .push(EntityLifecycleEvent::PassengersChanged {
                        vehicle_id,
                        passenger_ids,
                    });
            }
        }
        // `AbstractSchoolingFish.startFollowing` (`AbstractSchoolingFish.java:46-50`).
        if let Some(leader) = school_leader {
            crate::entity::fish::commit_school_follow(self, id, leader);
        }
        id
    }

    /// Forgets a fresh body that is never added, with every passenger it carries.
    pub fn discard_fresh(&mut self, entity: Box<Entity>) {
        let mut ids = vec![entity.id()];
        let mut index = 0;
        while let Some(&id) = ids.get(index) {
            ids.extend_from_slice(self.passengers(id));
            index += 1;
        }
        self.stop_invalid_riding(entity.id());
        self.ids_by_uuid.remove(&entity.uuid());
        for &id in &ids {
            if self.players.contains_key(&id) {
                self.set_vehicle(id, None);
                continue;
            }
            if let Some(passenger) = self.entities.remove(&id).and_then(|slot| slot.body) {
                self.live_body_count -= 1;
                self.unfile(id, passenger.filed_section());
                self.mark_chunk_dirty(passenger.filed_section().column());
                self.ids_by_uuid.remove(&passenger.uuid());
                self.unindex_body_metadata(&passenger);
            }
            self.ai_entity_ids.remove(&id);
            self.dragon_ids.remove(&id);
            self.visibility.forget(id);
        }
        self.retain_lifecycle_events(|event| match event {
            EntityLifecycleEvent::Added { entity, .. }
            | EntityLifecycleEvent::TrackingStarted { entity } => !ids.contains(&entity.id()),
            EntityLifecycleEvent::PassengersChanged { vehicle_id, .. } => !ids.contains(vehicle_id),
            _ => true,
        });
    }

    /// Adds a finalized secondary and links it to the fresh `root` as its
    /// vehicle or passenger; a secondary that cannot link is dropped.
    pub fn add_fresh_linked(
        &mut self,
        root: &mut Entity,
        secondary: Box<Entity>,
        secondary_is_vehicle: bool,
        forced: bool,
    ) -> Option<i32> {
        let id = self.add_fresh(secondary);
        let linked = if secondary_is_vehicle {
            self.vehicle(root.id()).is_none() && self.fresh_ride(root, id, forced)
        } else {
            self.fresh_carry(root, id, forced)
        };
        if linked {
            return Some(id);
        }
        if let Some(secondary) = self.take_fresh(id) {
            self.discard_fresh(secondary);
        }
        None
    }

    /// `Entity.startRiding` of a fresh body onto a vehicle in the store.
    pub fn fresh_ride(&mut self, fresh: &mut Entity, vehicle_id: i32, forced: bool) -> bool {
        self.link_fresh(fresh, fresh.id(), vehicle_id, forced)
    }

    /// `Entity.startRiding` of a stored passenger onto a fresh body.
    pub fn fresh_carry(&mut self, fresh: &mut Entity, passenger_id: i32, forced: bool) -> bool {
        self.link_fresh(fresh, passenger_id, fresh.id(), forced)
    }

    fn link_fresh(
        &mut self,
        fresh: &mut Entity,
        passenger_id: i32,
        vehicle_id: i32,
        forced: bool,
    ) -> bool {
        let admission = if forced {
            crate::entity::passenger::RelationAdmission::Forced
        } else {
            crate::entity::passenger::RelationAdmission::Normal
        };
        self.link_existing_passenger_with_admission(
            vehicle_id,
            passenger_id,
            admission,
            Some(fresh),
            None,
        )
    }
}
