use super::EntityStore;
use crate::entity::{Entity, EntityData};
use crate::world::simulation_level::LevelCtx;
use std::collections::BTreeSet;

fn is_mob(entity: &Entity) -> bool {
    matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_mob())
}

impl EntityStore {
    pub(crate) fn with_mob_ai_control<R>(
        &mut self,
        id: i32,
        control_owner: i32,
        run: impl FnOnce(
            &mut crate::entity::ai::mob::MobAiRuntime,
            Option<&mut crate::entity::ai::mob::MobAiRuntime>,
            &mut Self,
        ) -> R,
    ) -> Option<R> {
        let mut runtime = self.lend_mob_ai(id)?;
        let mut control = if id == control_owner {
            None
        } else {
            let Some(control) = self.lend_mob_ai(control_owner) else {
                self.return_mob_ai(id, runtime);
                return None;
            };
            Some(control)
        };
        let result = run(&mut runtime, control.as_deref_mut(), self);
        self.return_mob_ai(id, runtime);
        if let Some(control) = control {
            self.return_mob_ai(control_owner, control);
        }
        Some(result)
    }

    fn lend_mob_ai(&mut self, id: i32) -> Option<Box<crate::entity::ai::mob::MobAiRuntime>> {
        if !self.entities.get(&id).is_some_and(|slot| slot.ai.is_some()) {
            return None;
        }
        self.mark_entity_chunk_dirty(id);
        self.entities.get_mut(&id)?.ai.take()
    }

    fn return_mob_ai(&mut self, id: i32, runtime: Box<crate::entity::ai::mob::MobAiRuntime>) {
        if let Some(slot) = self.entities.get_mut(&id) {
            slot.ai = Some(runtime);
            self.mark_entity_chunk_dirty(id);
        }
    }

    ///
    /// The `instanceof PathfinderMob` gate is read as "owns a ground
    /// navigation": a mob this runtime never paths has no path to drop.
    pub fn stop_mob_navigation(&mut self, id: i32) -> bool {
        let Some(runtime) = self.mob_ai_mut(id) else {
            return false;
        };
        runtime.stop_navigation();
        true
    }

    pub(crate) fn refresh_retained_mob_targets(&mut self, id: i32) {
        if let Some(super::EntitySlot {
            body: Some(entity),
            ai: Some(runtime),
            ..
        }) = self.entities.get_mut(&id)
        {
            runtime.install_fox_target_goals(entity);
            if crate::entity::ai::mob::target::anger::is_neutral_mob(entity.entity_type_id()) {
                runtime.target_state_mut().load_target_pending = true;
            }
        }
    }

    pub(crate) fn refresh_retained_mob_navigation(&mut self, id: i32) {
        if let Some(super::EntitySlot {
            body: Some(entity),
            ai: Some(runtime),
            ..
        }) = self.entities.get_mut(&id)
        {
            if let Some(pos) = runtime.refresh_retained_navigation(entity) {
                self.lifecycle_events
                    .push(super::EntityLifecycleEvent::BlockDestruction {
                        entity_id: id,
                        pos,
                        stage: -1,
                    });
            }
        }
    }

    pub(crate) fn effective_pathfinding_malus(
        &self,
        requester: &Entity,
        level: &LevelCtx<'_, '_>,
    ) -> Option<crate::entity::ai::mob::navigation::malus::PathfindingMalus> {
        self.effective_pathfinding_malus_with_provenance(requester, level)
            .map(|(malus, _)| malus)
    }

    pub(crate) fn effective_pathfinding_malus_with_provenance(
        &self,
        requester: &Entity,
        level: &LevelCtx<'_, '_>,
    ) -> Option<(
        crate::entity::ai::mob::navigation::malus::PathfindingMalus,
        bool,
    )> {
        let mut own = self.mob_ai(requester.id())?.pathfinding_malus();
        crate::entity::creaking::apply_transient_pathfinding_malus(&mut own, requester);
        let Some(vehicle_id) = self.vehicle(requester.id()) else {
            return Some((own, false));
        };
        let vehicle = self.get(vehicle_id)?;
        if vehicle.entity_type_id() != bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_STRIDER_ID
            || self.passengers(vehicle_id).first() != Some(&requester.id())
        {
            return Some((own, false));
        }
        if self.controlling_mob_passenger_with_detached(vehicle, requester, level)?
            == Some(requester.id())
        {
            let mut riding = self.mob_ai(vehicle_id)?.pathfinding_malus();
            crate::entity::creaking::apply_transient_pathfinding_malus(&mut riding, vehicle);
            Some((riding, true))
        } else {
            Some((own, false))
        }
    }

    pub(crate) fn controlling_mob_passenger(
        &self,
        entity: &Entity,
        level: &LevelCtx<'_, '_>,
    ) -> Option<Option<i32>> {
        self.controlling_mob_passenger_with_detached(entity, entity, level)
    }

    fn controlling_mob_passenger_with_detached(
        &self,
        entity: &Entity,
        detached: &Entity,
        level: &LevelCtx<'_, '_>,
    ) -> Option<Option<i32>> {
        crate::entity::passenger::mob_controller(
            entity,
            Some(self),
            &level.capture_registry_tags(),
            Some(detached),
        )
    }

    pub(crate) fn mob_control_owner(
        &self,
        requester: &Entity,
        level: &LevelCtx<'_, '_>,
    ) -> Option<i32> {
        if self.vehicle(requester.id()).is_none() {
            return Some(requester.id());
        }
        let mut current = requester;
        let mut visited = BTreeSet::new();
        loop {
            if !visited.insert(current.id()) {
                return None;
            }
            let Some(vehicle_id) = self.vehicle(current.id()) else {
                return Some(current.id());
            };
            if self
                .passengers(vehicle_id)
                .iter()
                .filter(|&&id| id == current.id())
                .count()
                != 1
            {
                return None;
            }
            let vehicle = if vehicle_id == requester.id() {
                requester
            } else {
                self.get(vehicle_id)?
            };
            if !is_mob(vehicle) || self.passengers(vehicle_id).first() != Some(&current.id()) {
                return Some(current.id());
            }
            if self.controlling_mob_passenger_with_detached(vehicle, requester, level)?
                != Some(current.id())
            {
                return Some(current.id());
            }
            current = vehicle;
        }
    }
}

#[cfg(test)]
mod tests;
