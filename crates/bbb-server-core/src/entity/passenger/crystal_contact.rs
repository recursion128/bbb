use super::*;
use crate::entity::inside_blocks::NonlivingInsidePause;
use crate::world::simulation_level::LevelCtx;

pub struct PendingCrystalContactTick {
    inside: NonlivingInsidePause,
    task: TickTask,
    continuation: EntityTickContinuation,
    portal_allowed: bool,
}

impl PendingCrystalContactTick {
    pub(super) fn new(
        inside: NonlivingInsidePause,
        task: TickTask,
        continuation: EntityTickContinuation,
        portal_allowed: bool,
    ) -> Self {
        Self {
            inside,
            task,
            continuation,
            portal_allowed,
        }
    }

    pub fn victim_identity(&self) -> (i32, uuid::Uuid) {
        self.inside.victim_identity()
    }

    pub fn complete_hurt(&mut self, succeeded: bool) {
        self.inside.complete_hurt(succeeded);
    }

    pub fn hurt(&self) -> &crate::entity::projectile_action::PendingProjectileHurt {
        self.inside.hurt()
    }

    pub(crate) fn resume(
        self,
        store: &mut EntityStore,
        level: &mut LevelCtx<'_, '_>,
    ) -> Result<EntityTickOutcome, ()> {
        let Self {
            inside,
            task,
            continuation,
            portal_allowed,
        } = self;
        let (id, uuid) = inside.victim_identity();
        if store.get(id).is_none_or(|body| body.uuid() != uuid) {
            return Ok(finish_member(store, level, task, continuation));
        }

        let next = store
            .with_entity_mut(id, |body, store| inside.resume(body, store, level))
            .flatten()
            .ok_or(())?;
        match next {
            std::ops::ControlFlow::Break(inside) => Ok(EntityTickOutcome::CrystalContact(
                Box::new(Self::new(inside, task, continuation, portal_allowed)),
            )),
            std::ops::ControlFlow::Continue(()) => {
                finish_portal(store, level, id, uuid, task, continuation, portal_allowed)
            }
        }
    }
}

fn finish_portal(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    id: i32,
    uuid: uuid::Uuid,
    task: TickTask,
    continuation: EntityTickContinuation,
    portal_allowed: bool,
) -> Result<EntityTickOutcome, ()> {
    if store.get(id).is_none_or(|body| body.uuid() != uuid) {
        return Ok(finish_member(store, level, task, continuation));
    }

    level.begin_portal_tick(Some((
        portal_allowed,
        store
            .get(id)
            .map_or(300, |body| store.dimension_changing_delay(body)),
        true,
    )));
    store
        .with_entity_mut(id, |body, _| crate::entity::handle_portal(body, level))
        .ok_or(())?;
    if let Some(request) = level.take_portal_request() {
        return Ok(EntityTickOutcome::Portal(PendingPortalEntityTick::new(
            request,
            task,
            continuation,
        )));
    }
    Ok(finish_member(store, level, task, continuation))
}

fn finish_member(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    task: TickTask,
    mut continuation: EntityTickContinuation,
) -> EntityTickOutcome {
    finish_tick_member(store, task, &mut continuation);
    continue_entity_tick(store, level, continuation)
}
