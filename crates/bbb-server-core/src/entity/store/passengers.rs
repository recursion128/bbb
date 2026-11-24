use std::collections::BTreeSet;

use bbb_protocol::packets::Vec3d;

use super::{
    DetachedEntityBodies, Entity, EntityBodyReadView, EntityLifecycleEvent, EntityStore,
    ExternalPlayerDismount, ExternalPlayerVehicleUnload, RemovalReason,
};

impl EntityStore {
    pub fn vehicle(&self, passenger_id: i32) -> Option<i32> {
        self.entities.get(&passenger_id)?.vehicle
    }

    pub(crate) fn root_vehicle(&self, mut id: i32) -> i32 {
        while let Some(parent) = self.vehicle(id) {
            id = parent;
        }
        id
    }

    pub fn passengers(&self, vehicle_id: i32) -> &[i32] {
        self.entities
            .get(&vehicle_id)
            .map_or(&[], |slot| slot.passengers.as_slice())
    }

    pub(super) fn set_vehicle(&mut self, passenger_id: i32, vehicle: Option<i32>) {
        let previous = self.vehicle(passenger_id);
        if previous != vehicle {
            self.mark_entity_chunk_dirty(passenger_id);
            if let Some(previous_vehicle) = previous {
                self.mark_entity_chunk_dirty(previous_vehicle);
            }
            if let Some(next_vehicle) = vehicle {
                self.mark_entity_chunk_dirty(next_vehicle);
            }
        }
        if let Some(slot) = self.entities.get_mut(&passenger_id) {
            slot.vehicle = vehicle;
        }
    }

    pub(crate) fn passenger_relation_matches(
        &self,
        passenger_id: i32,
        vehicle: Option<i32>,
    ) -> bool {
        self.vehicle(passenger_id) == vehicle
            && vehicle.is_none_or(|id| {
                self.passengers(id)
                    .iter()
                    .filter(|listed| **listed == passenger_id)
                    .count()
                    == 1
            })
    }

    fn set_happy_ghast_still_timeout(&mut self, vehicle_id: i32, timeout: i32) {
        let packet = self.get_mut(vehicle_id).and_then(|entity| {
            crate::entity::happy_ghast::set_server_still_timeout(entity, timeout)
        });
        if let Some(packet) = packet {
            self.broadcast_to_tracking(vehicle_id, packet);
        }
    }

    pub(super) fn publish_passengers_changed(
        &mut self,
        vehicle_id: i32,
        previous_count: usize,
        detached: Option<&mut Entity>,
    ) {
        let passenger_ids = self.passengers(vehicle_id).to_vec();
        if (previous_count == 0) != passenger_ids.is_empty() {
            let sound = detached
                .or_else(|| self.get_mut(vehicle_id))
                .and_then(|entity| {
                    crate::entity::happy_ghast::on_passenger_transition(
                        entity,
                        !passenger_ids.is_empty(),
                    )
                });
            if let Some(sound) = sound {
                self.play_entity_sound(sound);
            }
        }
        self.lifecycle_events
            .push(EntityLifecycleEvent::PassengersChanged {
                vehicle_id,
                passenger_ids,
            });
    }

    pub fn has_indirect_passenger(&self, source_entity_id: i32, passenger_entity_id: i32) -> bool {
        let mut current = passenger_entity_id;
        let mut seen = BTreeSet::new();
        while let Some(vehicle_id) = self.vehicle(current) {
            if vehicle_id == source_entity_id {
                return true;
            }
            if !seen.insert(current) {
                return false;
            }
            current = vehicle_id;
        }
        false
    }

    pub(crate) fn checked_has_indirect_passenger(
        &self,
        body: &Entity,
        passenger_entity_id: i32,
    ) -> Option<bool> {
        let mut found = false;
        self.walk_checked_vehicle_chain(body, passenger_entity_id, true, |depth, id| {
            if depth > 0 && id == body.id() {
                found = true;
            }
        })?;
        Some(found)
    }

    pub(crate) fn checked_same_vehicle(&self, body: &Entity, other_id: i32) -> Option<bool> {
        let source_root = self.walk_checked_vehicle_chain(body, body.id(), false, |_, _| {})?;
        let other_root = self.walk_checked_vehicle_chain(body, other_id, false, |_, _| {})?;
        Some(source_root == other_root)
    }

    fn walk_checked_vehicle_chain(
        &self,
        body: &Entity,
        id: i32,
        known_session_start: bool,
        mut visit: impl FnMut(usize, i32),
    ) -> Option<i32> {
        let mut current = id;
        let mut depth = 0usize;
        let mut first = 0i32;
        let mut second = 0i32;
        let mut seen: Option<BTreeSet<i32>> = None;
        loop {
            match depth {
                0 => first = current,
                1 => {
                    if current == first {
                        return None;
                    }
                    second = current;
                }
                2 => {
                    if current == first || current == second {
                        return None;
                    }
                    seen = Some(BTreeSet::from([first, second, current]));
                }
                _ => {
                    if !seen.as_mut().expect("depth >= 3").insert(current) {
                        return None;
                    }
                }
            }
            visit(depth, current);
            let stored = current == body.id() || self.get(current).is_some();
            let external = self.is_external_player_passenger(current);
            let parent = self.vehicle(current);
            let incoming = parent.map_or(0, |vehicle| {
                self.passengers(vehicle)
                    .iter()
                    .filter(|id| **id == current)
                    .count()
            });
            // FactsReader validated this unmounted session; generic collision/tree IDs do not.
            if known_session_start
                && depth == 0
                && !stored
                && !external
                && incoming == 0
                && parent.is_none()
            {
                return Some(current);
            }
            if stored == external {
                return None;
            }
            let Some(parent) = parent else {
                return (incoming == 0 && !external).then_some(current);
            };
            if incoming != 1
                || !self.passengers(parent).contains(&current)
                || (parent != body.id() && self.get(parent).is_none())
            {
                return None;
            }
            current = parent;
            depth += 1;
        }
    }

    pub(crate) fn checked_teleport_passengers(
        &self,
        body: &Entity,
        parent: i32,
    ) -> Option<Vec<i32>> {
        let ids = self.passengers(parent);
        let unique: BTreeSet<_> = ids.iter().copied().collect();
        if unique.len() != ids.len() {
            return None;
        }
        for id in ids {
            let external = self.is_external_player_passenger(*id);
            if (external && (self.get(*id).is_some() || !self.passengers(*id).is_empty()))
                || (!external && self.get(*id).is_none())
                || !self.passenger_relation_matches(*id, Some(parent))
                || self
                    .walk_checked_vehicle_chain(body, *id, false, |_, _| {})
                    .is_none()
            {
                return None;
            }
        }
        Some(ids.to_vec())
    }

    pub fn attach_external_player(
        &mut self,
        player_entity_id: i32,
        vehicle_id: i32,
        can_ride: crate::entity::ExternalPlayerCanRide,
    ) -> bool {
        if can_ride != crate::entity::ExternalPlayerCanRide::Allowed {
            return false;
        }
        self.attach_external_player_with_admission(
            player_entity_id,
            vehicle_id,
            self.external_player_insertion_index(vehicle_id),
            crate::entity::passenger::RelationAdmission::Normal,
        )
    }

    /// Bypasses normal boarding restrictions for commands and restored seats.
    pub(crate) fn attach_external_player_forced(
        &mut self,
        player_entity_id: i32,
        vehicle_id: i32,
        passenger_index: usize,
    ) -> bool {
        self.attach_external_player_with_admission(
            player_entity_id,
            vehicle_id,
            passenger_index,
            crate::entity::passenger::RelationAdmission::Forced,
        )
    }

    pub(crate) fn external_player_insertion_index(&self, vehicle_id: i32) -> usize {
        let passengers = self.passengers(vehicle_id);
        if passengers
            .first()
            .is_some_and(|id| !self.is_external_player_passenger(*id))
        {
            0
        } else {
            passengers.len()
        }
    }

    fn attach_external_player_with_admission(
        &mut self,
        player_entity_id: i32,
        vehicle_id: i32,
        passenger_index: usize,
        admission: crate::entity::passenger::RelationAdmission,
    ) -> bool {
        let Some(vehicle) = self.get(vehicle_id) else {
            return false;
        };
        let vehicle_type_id = vehicle.entity_type_id();
        let vehicle_serializable =
            bbb_protocol::entity_types::vanilla_entity_type_is_serializable(vehicle_type_id)
                || self
                    .plugins()
                    .content_registry()
                    .entity_by_id(vehicle_type_id)
                    .is_some_and(|spec| spec.serializable);
        if !self.players.contains_key(&player_entity_id)
            || self.is_external_player_passenger(player_entity_id)
            || !vehicle_serializable
        {
            return false;
        }
        let passengers = self.passengers(vehicle_id);
        if passenger_index > passengers.len() {
            return false;
        }
        let passenger_count = passengers.len() + 1;
        let supported = match admission {
            crate::entity::passenger::RelationAdmission::Normal => {
                crate::entity::passenger::external_player_relation_supported(
                    crate::entity::passenger::RelationAdmission::Normal,
                    vehicle,
                    passenger_index,
                    passenger_count,
                    self.game_time,
                )
            }
            crate::entity::passenger::RelationAdmission::Forced => {
                crate::entity::passenger::external_player_relation_restorable(
                    vehicle,
                    passenger_index,
                    passenger_count,
                )
            }
        };
        if !supported {
            return false;
        }
        let mut ancestor = Some(vehicle_id);
        let mut seen = BTreeSet::new();
        while let Some(id) = ancestor {
            if id == player_entity_id || !seen.insert(id) {
                return false;
            }
            ancestor = self.vehicle(id);
        }
        let Some(timeout) = crate::entity::happy_ghast::added_passenger_timeout(
            vehicle,
            EntityBodyReadView::new(self, None, None),
            player_entity_id,
        ) else {
            return false;
        };
        self.slot_mut(player_entity_id).vehicle = Some(vehicle_id);
        let passengers = &mut self.slot_mut(vehicle_id).passengers;
        passengers.insert(passenger_index, player_entity_id);
        self.mark_entity_chunk_dirty(vehicle_id);
        if let Some(timeout) = timeout {
            self.set_happy_ghast_still_timeout(vehicle_id, timeout);
        }
        self.publish_passengers_changed(vehicle_id, passenger_count - 1, None);
        true
    }

    pub fn detach_external_player(&mut self, player_entity_id: i32) -> Option<i32> {
        let vehicle_id = self.external_player_structural_vehicle(player_entity_id)?;
        let previous_count = self.passengers(vehicle_id).len();
        let passenger_ids = self
            .passengers(vehicle_id)
            .iter()
            .copied()
            .filter(|id| *id != player_entity_id)
            .collect::<Vec<_>>();
        self.set_vehicle(player_entity_id, None);
        self.slot_mut(vehicle_id).passengers = passenger_ids;
        self.set_happy_ghast_still_timeout(vehicle_id, 10);
        self.publish_passengers_changed(vehicle_id, previous_count, None);
        self.publish_external_player_boarding_cooldown_reset(player_entity_id, None);
        Some(vehicle_id)
    }

    pub fn external_player_vehicle(&self, player_entity_id: i32) -> Option<i32> {
        self.external_player_structural_vehicle(player_entity_id)
    }

    pub fn external_player_root_vehicle(&self, player_entity_id: i32) -> Option<i32> {
        self.external_player_structural_root_vehicle(player_entity_id)
    }

    pub(crate) fn external_player_structural_root_vehicle(
        &self,
        player_entity_id: i32,
    ) -> Option<i32> {
        EntityBodyReadView::new(self, None, None).external_player_root_vehicle(player_entity_id)
    }

    pub fn external_player_direct_controlled_boat(&self, player_entity_id: i32) -> Option<i32> {
        let vehicle_id = self.geometry_valid_external_player_vehicle(player_entity_id)?;
        let passenger_index = self
            .passengers(vehicle_id)
            .iter()
            .position(|id| *id == player_entity_id)?;
        crate::entity::passenger::external_player_controls_boat(
            self.get(vehicle_id)?,
            passenger_index,
        )
        .then_some(vehicle_id)
    }

    pub fn external_player_vehicle_placement(
        &self,
        player_entity_id: i32,
        player_yaw: f32,
    ) -> Option<crate::entity::passenger::ExternalPlayerVehiclePlacement> {
        self.external_player_vehicle_placement_with_rider(
            player_entity_id,
            player_yaw,
            1.0,
            bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_STANDING_ID,
        )
    }

    pub fn external_player_vehicle_placement_with_rider(
        &self,
        player_entity_id: i32,
        player_yaw: f32,
        player_scale: f32,
        player_pose_id: i32,
    ) -> Option<crate::entity::passenger::ExternalPlayerVehiclePlacement> {
        let vehicle_id = self.geometry_valid_external_player_vehicle(player_entity_id)?;
        let passengers = self.passengers(vehicle_id);
        let passenger_index = passengers.iter().position(|id| *id == player_entity_id)?;
        crate::entity::passenger::external_player_placement(
            self.get(vehicle_id)?,
            passenger_index,
            passengers.len(),
            player_yaw,
            player_scale,
            player_pose_id,
            self.game_time,
        )
    }

    /// Test/back-compat seam. Production packet handling must use the typed
    /// collision-resolved external-player Boat control kernel.
    #[cfg(test)]
    pub(crate) fn update_external_player_root_controlled_boat_pose(
        &mut self,
        player_entity_id: i32,
        position: Vec3d,
        y_rot: f32,
        x_rot: f32,
    ) -> bool {
        if !position.x.is_finite()
            || !position.y.is_finite()
            || !position.z.is_finite()
            || !y_rot.is_finite()
            || !x_rot.is_finite()
        {
            return false;
        }
        let Some(vehicle_id) = self.external_player_direct_controlled_boat(player_entity_id) else {
            return false;
        };
        if self.external_player_root_vehicle(player_entity_id) != Some(vehicle_id) {
            return false;
        };
        let Some(vehicle) = self.get_mut(vehicle_id) else {
            return false;
        };
        vehicle.set_pos(position);
        vehicle.set_rot(y_rot, x_rot);
        self.refile(vehicle_id);
        true
    }

    /// Test/back-compat seam. Production packet handling must bind paddle
    /// input to an expected controller snapshot.
    #[cfg(test)]
    pub(crate) fn set_external_player_boat_paddles(
        &mut self,
        player_entity_id: i32,
        left: bool,
        right: bool,
    ) -> bool {
        let Some(vehicle_id) = self.external_player_direct_controlled_boat(player_entity_id) else {
            return false;
        };
        let Some(vehicle) = self.get_mut(vehicle_id) else {
            return false;
        };
        crate::entity::boat::set_paddle_state(vehicle, left, right);
        true
    }

    /// Applies the post-save mount half of `PlayerList.remove` without
    /// exposing a caller-controlled recursive entity removal operation.
    pub fn unload_external_player_vehicle(
        &mut self,
        player_entity_id: i32,
    ) -> Option<ExternalPlayerVehicleUnload> {
        let direct_vehicle_id = self.external_player_structural_vehicle(player_entity_id)?;
        let root_vehicle_id = self.external_player_structural_root_vehicle(player_entity_id)?;
        if self.external_player_count_in_tree(root_vehicle_id) != 1 {
            self.detach_external_player(player_entity_id)?;
            return Some(ExternalPlayerVehicleUnload::DetachedOnly {
                direct_vehicle_id,
                root_vehicle_id,
            });
        }
        self.detach_external_player(player_entity_id)?;
        let entity_ids = self.unload_player_entity_tree(root_vehicle_id)?;
        Some(ExternalPlayerVehicleUnload::UnloadedRootTree {
            direct_vehicle_id,
            root_vehicle_id,
            entity_ids,
        })
    }

    pub(crate) fn unload_player_entity_tree(&mut self, root_entity_id: i32) -> Option<Vec<i32>> {
        let entity_ids = self.entity_tree_ids(root_entity_id)?;
        for entity_id in &entity_ids {
            self.remove(*entity_id, RemovalReason::UnloadedWithPlayer);
        }
        Some(entity_ids)
    }

    pub(crate) fn entity_tree_ids(&self, root_entity_id: i32) -> Option<Vec<i32>> {
        fn visit(
            store: &EntityStore,
            entity_id: i32,
            visited: &mut BTreeSet<i32>,
            ids: &mut Vec<i32>,
        ) -> Option<()> {
            if !visited.insert(entity_id) || store.get(entity_id).is_none() {
                return None;
            }
            for passenger_id in store.passengers(entity_id) {
                if store.is_external_player_passenger(*passenger_id) {
                    continue;
                }
                if store.valid_vehicle(*passenger_id) != Some(entity_id) {
                    return None;
                }
                visit(store, *passenger_id, visited, ids)?;
            }
            ids.push(entity_id);
            Some(())
        }

        let mut ids = Vec::new();
        visit(self, root_entity_id, &mut BTreeSet::new(), &mut ids)?;
        Some(ids)
    }

    pub(crate) fn is_external_player_passenger(&self, entity_id: i32) -> bool {
        self.players.contains_key(&entity_id)
            && self
                .entities
                .get(&entity_id)
                .is_some_and(|slot| slot.vehicle.is_some())
    }

    pub(crate) fn external_player_passenger_ids(&self) -> impl Iterator<Item = i32> + '_ {
        self.players.keys().copied().filter(|id| {
            self.entities
                .get(id)
                .is_some_and(|slot| slot.vehicle.is_some())
        })
    }

    /// Files a test player under `player_entity_id` at the vehicle, then
    /// boards it.
    #[cfg(test)]
    pub(crate) fn attach_test_player(
        &mut self,
        player_entity_id: i32,
        vehicle_id: i32,
        can_ride: crate::entity::ExternalPlayerCanRide,
    ) -> bool {
        if !self.entities.contains_key(&player_entity_id) {
            let position = self
                .get(vehicle_id)
                .map_or_else(Vec3d::default, Entity::position);
            self.place_test_player(player_entity_id, position);
        }
        self.attach_external_player(player_entity_id, vehicle_id, can_ride)
    }

    #[cfg(test)]
    pub(crate) fn place_test_player(&mut self, player_entity_id: i32, position: Vec3d) {
        let player = crate::world::ServerWorld::detached_player_for_test(
            player_entity_id,
            position,
            bbb_protocol::game_types::GameType::Survival,
        );
        self.insert_player(player_entity_id, player.uuid(), Box::new(player));
    }

    pub(crate) fn first_passenger_is_external_player(&self, vehicle_id: i32) -> bool {
        self.first_external_player_passenger(vehicle_id).is_some()
    }

    /// `Entity.getDimensionChangingDelay`; a projectile waits 2 ticks
    /// (`Projectile.java:387-389`), a boat or minecart 10
    /// (`VehicleEntity.java:115-117`).
    pub fn dimension_changing_delay(&self, entity: &Entity) -> i32 {
        if entity.is_projectile_class() {
            2
        } else if crate::entity::boat::is_boat(entity.data())
            || crate::entity::minecart::is_minecart(entity.data())
            || self.first_passenger_is_external_player(entity.id())
        {
            10
        } else {
            300
        }
    }

    pub(crate) fn first_external_player_passenger(&self, vehicle_id: i32) -> Option<i32> {
        self.passengers(vehicle_id)
            .first()
            .copied()
            .filter(|player_id| {
                self.external_player_structural_vehicle(*player_id) == Some(vehicle_id)
            })
    }

    pub(crate) fn has_external_player_passenger(&self, vehicle_id: i32) -> bool {
        self.passengers(vehicle_id).iter().any(|player_id| {
            self.external_player_structural_vehicle(*player_id) == Some(vehicle_id)
        })
    }

    pub(crate) fn eject_passengers_reverse(&mut self, vehicle_id: i32) -> bool {
        let position = self.get(vehicle_id).map(Entity::position);
        self.eject_passengers_reverse_at(vehicle_id, position, None)
    }

    pub(crate) fn eject_body_passengers_reverse(&mut self, vehicle: &mut Entity) -> bool {
        self.eject_passengers_reverse_at(vehicle.id(), Some(vehicle.position()), Some(vehicle))
    }

    fn eject_passengers_reverse_at(
        &mut self,
        vehicle_id: i32,
        position: Option<Vec3d>,
        mut detached: Option<&mut Entity>,
    ) -> bool {
        while let Some(passenger_id) = self
            .entities
            .get_mut(&vehicle_id)
            .and_then(|slot| slot.passengers.pop())
        {
            if let Some(body) = detached.as_deref() {
                self.mark_chunk_dirty(body.filed_section().column());
            }
            let was_external = self.is_external_player_passenger(passenger_id);
            self.set_vehicle(passenger_id, None);
            if !was_external {
                self.set_stored_passenger_boarding_cooldown(passenger_id);
            }
            if let Some(vehicle) = detached
                .as_deref_mut()
                .filter(|body| body.id() == vehicle_id)
            {
                crate::entity::happy_ghast::publish_still_timeout(vehicle, self, 10);
            } else {
                self.set_happy_ghast_still_timeout(vehicle_id, 10);
            }
            self.publish_passengers_changed(
                vehicle_id,
                self.passengers(vehicle_id).len() + 1,
                detached.as_deref_mut(),
            );
            if was_external {
                self.publish_external_player_boarding_cooldown_reset(
                    passenger_id,
                    position.map(|vehicle_position| ExternalPlayerDismount {
                        vehicle_id,
                        vehicle_position,
                    }),
                );
            }
        }
        true
    }

    fn geometry_valid_external_player_vehicle(&self, player_entity_id: i32) -> Option<i32> {
        let vehicle_id = self.external_player_structural_vehicle(player_entity_id)?;
        let vehicle = self.get(vehicle_id)?;
        let passengers = self.passengers(vehicle_id);
        let passenger_index = passengers.iter().position(|id| *id == player_entity_id)?;
        crate::entity::passenger::external_player_relation_supported(
            crate::entity::passenger::RelationAdmission::Forced,
            vehicle,
            passenger_index,
            passengers.len(),
            self.game_time,
        )
        .then_some(vehicle_id)
    }

    pub(crate) fn external_player_structural_vehicle(&self, player_entity_id: i32) -> Option<i32> {
        EntityBodyReadView::new(self, None, None).external_player_vehicle(player_entity_id)
    }

    pub(crate) fn external_player_count_in_tree(&self, root_vehicle_id: i32) -> usize {
        EntityBodyReadView::new(self, None, None).external_player_count_in_tree(root_vehicle_id)
    }

    pub(crate) fn valid_vehicle(&self, passenger_id: i32) -> Option<i32> {
        EntityBodyReadView::new(self, None, None).valid_vehicle(passenger_id)
    }

    pub(crate) fn stop_invalid_riding(&mut self, passenger_id: i32) -> bool {
        let was_external = self.is_external_player_passenger(passenger_id);
        let Some(vehicle_id) = self.vehicle(passenger_id) else {
            return false;
        };
        let dismount = was_external
            .then(|| self.get(vehicle_id))
            .flatten()
            .map(|vehicle| ExternalPlayerDismount {
                vehicle_id,
                vehicle_position: vehicle.position(),
            });
        self.set_vehicle(passenger_id, None);
        if let Some(slot) = self.entities.get_mut(&vehicle_id) {
            let previous_count = slot.passengers.len();
            slot.passengers.retain(|id| *id != passenger_id);
            self.set_happy_ghast_still_timeout(vehicle_id, 10);
            self.publish_passengers_changed(vehicle_id, previous_count, None);
        }
        {
            if was_external {
                self.publish_external_player_boarding_cooldown_reset(passenger_id, dismount);
            } else {
                self.set_stored_passenger_boarding_cooldown(passenger_id);
            }
        }
        true
    }

    fn set_stored_passenger_boarding_cooldown(&mut self, passenger_id: i32) {
        if let Some(passenger) = self.get_mut(passenger_id) {
            passenger.set_boarding_cooldown(60);
        }
    }

    fn publish_external_player_boarding_cooldown_reset(
        &mut self,
        player_entity_id: i32,
        dismount: Option<ExternalPlayerDismount>,
    ) {
        self.lifecycle_events
            .push(EntityLifecycleEvent::ExternalPlayerBoardingCooldownReset {
                player_entity_id,
                dismount,
            });
    }

    pub fn link_existing_passenger(&mut self, vehicle_id: i32, passenger_id: i32) -> bool {
        self.link_existing_passenger_with_admission(
            vehicle_id,
            passenger_id,
            crate::entity::passenger::RelationAdmission::Normal,
            None,
            None,
        )
    }

    pub fn link_existing_passenger_forced(&mut self, vehicle_id: i32, passenger_id: i32) -> bool {
        self.link_existing_passenger_with_admission(
            vehicle_id,
            passenger_id,
            crate::entity::passenger::RelationAdmission::Forced,
            None,
            None,
        )
    }

    pub(crate) fn link_detached_passenger(
        &mut self,
        passenger: &mut Entity,
        vehicle_id: i32,
        ancestors: Option<&DetachedEntityBodies<'_>>,
    ) -> bool {
        if !self.is_detached(passenger.id()) {
            return false;
        }
        self.link_existing_passenger_with_admission(
            vehicle_id,
            passenger.id(),
            crate::entity::passenger::RelationAdmission::Normal,
            Some(passenger),
            ancestors,
        )
    }

    pub(crate) fn link_passenger_to_detached_vehicle(
        &mut self,
        vehicle: &mut Entity,
        passenger_id: i32,
        ancestors: Option<&DetachedEntityBodies<'_>>,
    ) -> bool {
        if !self.is_detached(vehicle.id()) {
            return false;
        }
        self.link_existing_passenger_with_admission(
            vehicle.id(),
            passenger_id,
            crate::entity::passenger::RelationAdmission::Normal,
            Some(vehicle),
            ancestors,
        )
    }

    pub(super) fn link_existing_passenger_with_admission(
        &mut self,
        vehicle_id: i32,
        passenger_id: i32,
        admission: crate::entity::passenger::RelationAdmission,
        mut detached: Option<&mut Entity>,
        ancestors: Option<&DetachedEntityBodies<'_>>,
    ) -> bool {
        if !self.entities.contains_key(&vehicle_id) || !self.entities.contains_key(&passenger_id) {
            return false;
        }
        let bodies = EntityBodyReadView::new(self, detached.as_deref(), ancestors);
        let (Some(vehicle), Some(passenger)) = (bodies.get(vehicle_id), bodies.get(passenger_id))
        else {
            return false;
        };
        if vehicle_id == passenger_id || self.vehicle(passenger_id).is_some() {
            return false;
        }
        // `Entity.canRide` (`Entity.java:2393-2395`) and its refusals in
        // `Warden.java:169-171`, `WitherBoss.java:548`, `EnderDragon.java:840`.
        if admission == crate::entity::passenger::RelationAdmission::Normal
            && (passenger.boarding_cooldown() > 0
                || matches!(
                    passenger.entity_type_id(),
                    bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WARDEN_ID
                        | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WITHER_ID
                        | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID
                ))
        {
            return false;
        }
        let passenger_index = self.passengers(vehicle_id).len();
        if !crate::entity::passenger::relation_supported(
            admission,
            vehicle,
            passenger,
            passenger_index,
            passenger_index + 1,
            self.game_time,
        ) {
            return false;
        }
        let mut ancestor = Some(vehicle_id);
        let mut seen = BTreeSet::new();
        while let Some(id) = ancestor {
            if id == passenger_id || !seen.insert(id) {
                return false;
            }
            ancestor = self.vehicle(id);
        }
        let Some(timeout) =
            crate::entity::happy_ghast::added_passenger_timeout(vehicle, bodies, passenger_id)
        else {
            return false;
        };
        self.slot_mut(passenger_id).vehicle = Some(vehicle_id);
        let passengers = &mut self.slot_mut(vehicle_id).passengers;
        passengers.push(passenger_id);
        self.mark_entity_chunk_dirty(passenger_id);
        self.mark_entity_chunk_dirty(vehicle_id);
        if let Some(body) = detached.as_deref() {
            self.mark_chunk_dirty(body.filed_section().column());
        }
        if let Some(timeout) = timeout {
            if let Some(vehicle) = detached
                .as_deref_mut()
                .filter(|body| body.id() == vehicle_id)
            {
                crate::entity::happy_ghast::publish_still_timeout(vehicle, self, timeout);
            } else {
                self.set_happy_ghast_still_timeout(vehicle_id, timeout);
            }
        }
        self.publish_passengers_changed(
            vehicle_id,
            passenger_index,
            detached.filter(|body| body.id() == vehicle_id),
        );
        true
    }

    pub(super) fn unlink_entity(&mut self, id: i32, mut detached: Option<&mut Entity>) {
        debug_assert!(detached.as_ref().is_none_or(|entity| entity.id() == id));
        if self.stop_invalid_riding(id) {
            if let Some(entity) = detached.as_deref_mut() {
                entity.set_boarding_cooldown(60);
            }
        }
        let dismount =
            detached
                .as_deref()
                .or_else(|| self.get(id))
                .map(|vehicle| ExternalPlayerDismount {
                    vehicle_id: id,
                    vehicle_position: vehicle.position(),
                });
        let passengers = self
            .entities
            .get_mut(&id)
            .map(|slot| std::mem::take(&mut slot.passengers))
            .unwrap_or_default();
        if !passengers.is_empty() {
            if let Some(body) = detached.as_deref() {
                self.mark_chunk_dirty(body.filed_section().column());
            }
            let previous_count = passengers.len();
            let mut external_players = Vec::new();
            for passenger_id in passengers {
                let was_external = self.is_external_player_passenger(passenger_id);
                self.set_vehicle(passenger_id, None);
                if let Some(entity) = detached.as_deref_mut() {
                    crate::entity::happy_ghast::publish_still_timeout(entity, self, 10);
                }
                if was_external {
                    external_players.push(passenger_id);
                } else {
                    self.set_stored_passenger_boarding_cooldown(passenger_id);
                }
            }
            self.publish_passengers_changed(id, previous_count, detached);
            for player_entity_id in external_players {
                self.publish_external_player_boarding_cooldown_reset(player_entity_id, dismount);
            }
        }
    }
}
