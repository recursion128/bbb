//! Autonomous baby hoglin riding.

use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_HOGLIN_ID, VANILLA_ENTITY_TYPE_PIGLIN_ID};
use bbb_server_types::geometry::Vec3Ops;

use crate::entity::ai::mob::target::TargetRef;
use crate::entity::store::EntityBodyReadView;
use crate::entity::{passenger, Entity, EntityData, EntityStore};
use crate::world::simulation_level::LevelCtx;

use super::{baby, celebration, is_admiring};

#[derive(Clone, Default)]
pub(super) struct RideState {
    start_delay: u16,
    ride: Option<(TargetRef, u16)>,
}

impl RideState {
    pub(super) fn target(&self) -> Option<TargetRef> {
        self.ride.map(|(target, _)| target)
    }
}

fn state(store: &EntityStore, id: i32) -> Option<&RideState> {
    Some(&store.mob_ai(id)?.piglin()?.riding)
}

fn state_mut(store: &mut EntityStore, id: i32) -> Option<&mut RideState> {
    Some(&mut store.mob_ai_mut(id)?.piglin_mut()?.riding)
}

fn eligible(body: &Entity, store: &EntityStore) -> bool {
    !is_admiring(body)
        && celebration(body).is_none()
        && store.mob_ai(body.id()).is_some_and(|runtime| {
            runtime.target_ref().is_none()
                && runtime
                    .piglin()
                    .is_some_and(|piglin| piglin.avoid_target.is_none())
        })
}

// PiglinAi.java:95-97; PiglinAi.java:665-672; CopyMemoryWithExpiry.java:13-19.
pub(super) fn sense(
    body: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    nearest_baby_hoglin: Option<TargetRef>,
) {
    if !baby(body) || !eligible(body, store) {
        return;
    }
    let Some(target) = nearest_baby_hoglin else {
        return;
    };
    let bodies = EntityBodyReadView::new(store, Some(body), level.detached_bodies());
    if !bodies.get(target.entity_id).is_some_and(|hoglin| {
        hoglin.uuid() == target.uuid
            && hoglin.entity_type_id() == VANILLA_ENTITY_TYPE_HOGLIN_ID
            && hoglin.is_alive_with_health()
            && baby(hoglin)
    }) {
        return;
    }
    let Some(riding) = state(store, body.id()) else {
        return;
    };
    if riding.ride.is_some() {
        return;
    }
    let delay = riding.start_delay;
    let next_delay = if delay == 0 {
        store.random().next_int_between_inclusive(200, 800) as u16
    } else {
        delay.saturating_sub(20)
    };
    let ride = (delay > 0 && next_delay == 0).then(|| {
        (
            target,
            store.random().next_int_between_inclusive(200, 600) as u16,
        )
    });
    if let Some(riding) = state_mut(store, body.id()) {
        riding.start_delay = next_delay;
        riding.ride = ride;
    }
}

fn valid_vehicle(
    body: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    target: TargetRef,
) -> bool {
    let bodies = EntityBodyReadView::new(store, Some(body), level.detached_bodies());
    let vehicle = if let Some(vehicle) = store.vehicle(body.id()) {
        bodies.get(vehicle)
    } else {
        bodies
            .get(target.entity_id)
            .filter(|vehicle| vehicle.uuid() == target.uuid)
    };
    let Some(vehicle) = vehicle else {
        return false;
    };
    // DismountOrSkipMounting.java:18-30.
    if !vehicle.is_alive_with_health()
        || body.position().distance_to_sqr(vehicle.position()) >= 64.0
    {
        return false;
    }
    // PiglinAi.java:487-494; PiglinAi.java:843-845.
    !matches!(vehicle.data(), EntityData::GeneralLiving(data) if data.is_mob())
        || (baby(vehicle)
            && !body
                .living()
                .is_some_and(|living| living.last_damage_type.is_some())
            && !vehicle
                .living()
                .is_some_and(|living| living.last_damage_type.is_some())
            && (vehicle.entity_type_id() != VANILLA_ENTITY_TYPE_PIGLIN_ID
                || store.vehicle(vehicle.id()).is_some()))
}

fn clear(body: &mut Entity, store: &mut EntityStore, level: &mut LevelCtx<'_, '_>, stop: bool) {
    let Some(riding) = state_mut(store, body.id()) else {
        return;
    };
    if riding.ride.take().is_none() {
        return;
    }
    if stop {
        if let Some(runtime) = store.mob_ai_mut(body.id()) {
            runtime.stop_navigation();
        }
    }
    passenger::stop_riding_detached(body, store, level);
}

fn reconcile(
    body: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    before_ai: bool,
) -> Option<TargetRef> {
    let (target, ticks) = state(store, body.id())?.ride?;
    let active = eligible(body, store);
    // PiglinAi.java:306-320: higher activities end the autonomous ride.
    if ticks == 0 || !active || !valid_vehicle(body, store, level, target) {
        // Post-AI priority changes may already have installed a different route.
        clear(body, store, level, before_ai || active);
        return None;
    }
    Some(target)
}

pub(super) fn before_ai(body: &mut Entity, store: &mut EntityStore, level: &mut LevelCtx<'_, '_>) {
    let _ = reconcile(body, store, level, true);
}

pub(super) fn tick(body: &mut Entity, store: &mut EntityStore, level: &mut LevelCtx<'_, '_>) {
    if let Some((_, ticks)) = state_mut(store, body.id()).and_then(|riding| riding.ride.as_mut()) {
        *ticks = ticks.saturating_sub(1);
    }
    let Some(target) = reconcile(body, store, level, false) else {
        return;
    };
    if store.vehicle(body.id()).is_some() {
        return;
    }
    let vehicle = {
        let bodies = EntityBodyReadView::new(store, Some(body), level.detached_bodies());
        let Some(root) = bodies
            .get(target.entity_id)
            .filter(|root| root.uuid() == target.uuid)
        else {
            return;
        };
        // Mount.java:14-25: approach the root hoglin, not its top rider.
        if body.position().distance_to_sqr(root.position()) >= 1.0 {
            return;
        }
        let mut vehicle = root.id();
        // Piglin.java:404-414: normal capacity admits at most three stacked riders.
        if baby(body) && root.entity_type_id() == VANILLA_ENTITY_TYPE_HOGLIN_ID {
            for _ in 0..2 {
                let Some(&first) = store.passengers(vehicle).first() else {
                    break;
                };
                if bodies.get(first).is_none() {
                    return;
                }
                vehicle = first;
            }
        }
        vehicle
    };
    if passenger::start_riding_detached(body, store, level, vehicle) {
        if let Some(runtime) = store.mob_ai_mut(body.id()) {
            runtime.stop_navigation();
        }
    }
}
