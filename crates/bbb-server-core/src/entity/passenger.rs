use crate::world::simulation_level::LevelCtx;
use std::collections::{BTreeSet, VecDeque};

use bbb_protocol::packets::Vec3d;

use crate::entity::portal::PortalTransitionRequest;
use crate::entity::store::{EntityBodyReadView, EntityStore};
use crate::entity::{Entity, EntityData};

mod crystal_contact;
pub use crystal_contact::PendingCrystalContactTick;
mod player_control;
pub(crate) mod teleport;
pub use teleport::{
    ExternalPlayerSnap, ExternalPlayerTeleportBody, TeleportApplication, TeleportTree,
};
mod vehicle_geometry;

pub(crate) use player_control::direct_controlling_player_with_resolver;
pub(crate) use player_control::mob_controller;
pub(crate) use player_control::{
    direct_controlling_living_with_registry_tags, DirectLivingController,
};
pub use player_control::{direct_controlling_player_with_registry_tags, PlayerIdentity};

pub use vehicle_geometry::ExternalPlayerVehiclePlacement;
pub(crate) use vehicle_geometry::{
    external_player_controls_boat, external_player_placement, external_player_relation_restorable,
    external_player_relation_supported, relation_supported, RelationAdmission,
};

pub(crate) fn start_riding_detached(
    body: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    vehicle_id: i32,
) -> bool {
    let Some(position) = EntityBodyReadView::new(store, Some(body), level.detached_bodies())
        .get(vehicle_id)
        .map(Entity::position)
    else {
        return false;
    };
    if !store.link_detached_passenger(body, vehicle_id, level.detached_bodies()) {
        return false;
    }
    // Entity.java:2377-2381.
    body.set_pose_id(bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_STANDING_ID);
    level.game_event_at(
        crate::level_event::VanillaGameEvent::ENTITY_MOUNT,
        position,
        Some(body.id()),
    );
    true
}

/// `passenger.startRiding(vehicle)` with the vehicle being the detached,
/// ticking body.
pub(crate) fn seat_on_detached_vehicle(
    vehicle: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    passenger_id: i32,
) -> bool {
    if !store.link_passenger_to_detached_vehicle(vehicle, passenger_id, level.detached_bodies()) {
        return false;
    }
    if let Some(passenger) = store.get_mut(passenger_id) {
        passenger.set_pose_id(bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_STANDING_ID);
    }
    level.game_event_at(
        crate::level_event::VanillaGameEvent::ENTITY_MOUNT,
        vehicle.position(),
        Some(passenger_id),
    );
    true
}

pub(crate) fn stop_riding_detached(
    body: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    let Some(vehicle_id) = store.vehicle(body.id()) else {
        return false;
    };
    let vehicle =
        EntityBodyReadView::new(store, Some(body), level.detached_bodies()).get(vehicle_id);
    let position = vehicle.map(Entity::position);
    // LivingEntity.java:2301-2323.
    let landing = vehicle.map(|vehicle| {
        if body.removal_reason().is_some() {
            (body.position(), body.pose_id())
        } else {
            crate::world::dismount_search::detached_mob_dismount_target(level, vehicle, body)
        }
    });
    if !store.stop_invalid_riding(body.id()) {
        return false;
    }
    // Entity.java:2449.
    body.set_boarding_cooldown(60);
    if let Some((landing, pose)) = landing {
        body.set_pose_id(pose);
        body.set_pos(landing);
    }
    // Entity.java:2403-2410.
    if body
        .removal_reason()
        .is_none_or(|reason| reason.should_destroy())
    {
        if let Some(position) = position {
            level.game_event_at(
                crate::level_event::VanillaGameEvent::ENTITY_DISMOUNT,
                position,
                Some(body.id()),
            );
        }
    }
    true
}

// Entity.java:3532-3534.
pub(crate) fn default_dismount_position(position: Vec3d, max_y: f64) -> Vec3d {
    Vec3d {
        y: max_y,
        ..position
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalPlayerCanRide {
    Allowed,
    Denied,
}

/// Opaque remainder of Vanilla's root/passenger preorder after a paused tick.
pub struct EntityTickContinuation {
    tasks: VecDeque<TickTask>,
    visited: BTreeSet<i32>,
    world_callbacks: bool,
}

/// One entity-root tick, either complete or paused at a Nether transition.
pub enum EntityTickOutcome {
    /// The entity tick completed.
    Complete,
    /// World ownership must resolve the requested transition.
    Portal(PendingPortalEntityTick),
    CrystalContact(Box<PendingCrystalContactTick>),
    Refused,
}

/// An entity tick paused at Vanilla's `Entity.handlePortal` position.
pub struct PendingPortalEntityTick {
    request: PortalTransitionRequest,
    task: TickTask,
    continuation: EntityTickContinuation,
}

impl PendingPortalEntityTick {
    fn new(
        request: PortalTransitionRequest,
        task: TickTask,
        continuation: EntityTickContinuation,
    ) -> Self {
        Self {
            request,
            task,
            continuation,
        }
    }

    /// The entity whose `handlePortal` produced this request.
    pub const fn requester_entity_id(&self) -> i32 {
        self.task.entity_id
    }

    /// The world transition requested by the entity processor.
    pub const fn request(&self) -> PortalTransitionRequest {
        self.request
    }

    pub fn finish_moved(self) -> EntityTickContinuation {
        self.continuation
    }

    pub fn finish_stayed(self, store: &mut EntityStore) -> EntityTickContinuation {
        let mut continuation = self.continuation;
        finish_tick_member(store, self.task, &mut continuation);
        continuation
    }
}

#[derive(Clone, Copy)]
struct TickTask {
    entity_id: i32,
    vehicle_id: Option<i32>,
}

enum TickTasksOutcome {
    Complete,
    Portal(TickTask, PortalTransitionRequest),
    CrystalContact(
        Box<crate::entity::inside_blocks::NonlivingInsidePause>,
        TickTask,
        bool,
    ),
}

#[cfg(test)]
pub(super) fn tick_root(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    id: i32,
) -> EntityTickOutcome {
    match tick_root_with_callbacks(store, level, id, false) {
        EntityTickOutcome::Refused => EntityTickOutcome::Complete,
        outcome => outcome,
    }
}

pub(super) fn tick_root_checked(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    id: i32,
) -> EntityTickOutcome {
    tick_root_with_callbacks(store, level, id, true)
}

fn tick_root_with_callbacks(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    id: i32,
    world_callbacks: bool,
) -> EntityTickOutcome {
    if !store.is_ticking(id)
        || store
            .get(id)
            .is_none_or(|entity| entity.removal_reason().is_some())
    {
        return EntityTickOutcome::Complete;
    }
    if let Some(vehicle) = store.valid_vehicle(id) {
        if store
            .get(vehicle)
            .is_some_and(|entity| entity.removal_reason().is_none())
        {
            check_outer_despawn(store, level, id);
            return EntityTickOutcome::Complete;
        }
    }

    if store.passengers(id).is_empty() && !store.is_external_player_passenger(id) {
        let task = TickTask {
            entity_id: id,
            vehicle_id: None,
        };
        let (ticked, portal_request, contact) =
            match crate::entity::tick_member(store, level, id, true, true, world_callbacks) {
                Ok(outcome) => outcome,
                Err(()) => return EntityTickOutcome::Refused,
            };
        let solo_continuation = || {
            let mut visited = BTreeSet::new();
            visited.insert(id);
            EntityTickContinuation {
                tasks: VecDeque::new(),
                visited,
                world_callbacks,
            }
        };
        if let Some(contact) = contact {
            return EntityTickOutcome::CrystalContact(Box::new(PendingCrystalContactTick::new(
                contact,
                task,
                solo_continuation(),
                true,
            )));
        }
        if !ticked {
            return EntityTickOutcome::Complete;
        }
        if let Some(portal_request) = portal_request {
            return EntityTickOutcome::Portal(PendingPortalEntityTick::new(
                portal_request,
                task,
                solo_continuation(),
            ));
        }
        if store.get(id).is_some() && !store.passengers(id).is_empty() {
            let mut continuation = solo_continuation();
            finish_tick_member(store, task, &mut continuation);
            return tick_segment(store, level, continuation);
        }
        return EntityTickOutcome::Complete;
    }

    let continuation = EntityTickContinuation {
        tasks: VecDeque::from([TickTask {
            entity_id: id,
            vehicle_id: None,
        }]),
        visited: BTreeSet::new(),
        world_callbacks,
    };
    tick_segment(store, level, continuation)
}

/// Resumes the remaining members of a root/passenger preorder after a transfer.
pub(crate) fn continue_entity_tick(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    continuation: EntityTickContinuation,
) -> EntityTickOutcome {
    tick_segment(store, level, continuation)
}

/// Resumes the preorder after the world refused the requester's transition.
pub(crate) fn continue_entity_tick_after_portal_refusal(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    continuation: EntityTickContinuation,
    _requester_entity_id: i32,
) -> EntityTickOutcome {
    tick_segment(store, level, continuation)
}

fn tick_segment(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    mut continuation: EntityTickContinuation,
) -> EntityTickOutcome {
    if continuation.tasks.is_empty() {
        return EntityTickOutcome::Complete;
    }
    let tasks_outcome = match tick_tasks(store, level, &mut continuation) {
        Ok(outcome) => outcome,
        Err(()) => {
            return EntityTickOutcome::Refused;
        }
    };
    match tasks_outcome {
        TickTasksOutcome::Portal(task, request) => {
            return EntityTickOutcome::Portal(PendingPortalEntityTick::new(
                request,
                task,
                continuation,
            ));
        }
        TickTasksOutcome::CrystalContact(inside, task, portal_allowed) => {
            return EntityTickOutcome::CrystalContact(Box::new(PendingCrystalContactTick::new(
                *inside,
                task,
                continuation,
                portal_allowed,
            )));
        }
        TickTasksOutcome::Complete => {}
    }
    EntityTickOutcome::Complete
}

fn check_outer_despawn(store: &mut EntityStore, level: &mut LevelCtx<'_, '_>, id: i32) {
    let Some(mut entity) = store.detach(id) else {
        return;
    };
    match entity.data() {
        EntityData::GeneralLiving(_) => {
            crate::entity::general_living::check_despawn(&mut entity, store, level)
        }
        EntityData::ShulkerBullet(_) => {
            crate::entity::shulker_bullet::check_despawn(&mut entity, store.difficulty())
        }
        _ => {}
    }
    store.attach(entity);
}

fn tick_tasks(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    continuation: &mut EntityTickContinuation,
) -> Result<TickTasksOutcome, ()> {
    while let Some(task) = continuation.tasks.pop_front() {
        let id = task.entity_id;
        let vehicle_id = task.vehicle_id;
        if store
            .get(id)
            .is_some_and(|entity| entity.removal_reason().is_some())
        {
            if vehicle_id.is_some() {
                store.stop_invalid_riding(id);
            }
            continue;
        }
        if store.is_external_player_passenger(id) {
            if vehicle_id.is_none() || store.external_player_structural_vehicle(id) != vehicle_id {
                store.stop_invalid_riding(id);
            }
            continue;
        }
        if let Some(vehicle_id) = vehicle_id {
            if continuation.visited.contains(&id)
                || store.valid_vehicle(id) != Some(vehicle_id)
                || passenger_placement(store, vehicle_id, id).is_none()
                || store.get(id).is_none()
            {
                store.stop_invalid_riding(id);
                continue;
            }
        } else if continuation.visited.contains(&id) || store.get(id).is_none() {
            store.stop_invalid_riding(id);
            continue;
        }
        if !store.is_ticking(id) {
            continue;
        }
        continuation.visited.insert(id);
        if vehicle_id.is_some() {
            if let Some(passenger) = store.get_mut(id) {
                passenger.set_delta_movement(Vec3d {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                });
            }
        }
        let root = vehicle_id.is_none();
        let (ticked, portal_request, contact) =
            crate::entity::tick_member(store, level, id, root, true, continuation.world_callbacks)?;
        if let Some(contact) = contact {
            return Ok(TickTasksOutcome::CrystalContact(
                Box::new(contact),
                task,
                vehicle_id.is_none(),
            ));
        }
        if !ticked {
            continue;
        }
        if let Some(portal_request) = portal_request {
            // `Entity.teleportCrossDimension` moves this entity and its descendants
            // before `ServerLevel.tickPassenger` can recurse into the old subtree.
            return Ok(TickTasksOutcome::Portal(task, portal_request));
        }
        finish_tick_member(store, task, continuation);
    }
    Ok(TickTasksOutcome::Complete)
}

fn finish_tick_member(
    store: &mut EntityStore,
    task: TickTask,
    continuation: &mut EntityTickContinuation,
) {
    let id = task.entity_id;
    let vehicle_id = task.vehicle_id;
    if store.get(id).is_none() {
        return;
    }
    if let Some(vehicle_id) = vehicle_id {
        if store.valid_vehicle(id) == Some(vehicle_id) {
            let Some(placement) = passenger_placement(store, vehicle_id, id) else {
                store.stop_invalid_riding(id);
                return;
            };
            if let Some(passenger) = store.get_mut(id) {
                placement.apply(passenger);
            }
            store.refile(id);
        }
    }

    let passengers = store.passengers(id).to_vec();
    for passenger_id in passengers.into_iter().rev() {
        continuation.tasks.push_front(TickTask {
            entity_id: passenger_id,
            vehicle_id: Some(id),
        });
    }
}

fn passenger_placement(
    store: &EntityStore,
    vehicle_id: i32,
    passenger_id: i32,
) -> Option<vehicle_geometry::PassengerPlacement> {
    let vehicle = store.get(vehicle_id)?;
    let passenger = store.get(passenger_id)?;
    let passengers = store.passengers(vehicle_id);
    let passenger_index = passengers.iter().position(|id| *id == passenger_id)?;
    vehicle_geometry::placement(
        vehicle,
        passenger,
        passenger_index,
        passengers.len(),
        store.game_time(),
    )
}
