//! Live target setters and neutral anger updates.
use super::anger::NeutralAnger;
use super::{TargetRef, TargetView};
use crate::entity::ai::mob::sensing::Sensing;
use crate::entity::{Entity, EntityData, EntityStore};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ENDERMAN_ID, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
    VANILLA_ENTITY_TYPE_POLAR_BEAR_ID, VANILLA_ENTITY_TYPE_WOLF_ID,
};
use bbb_protocol::packets::BlockPos;
#[cfg(test)]
use bbb_protocol::NbtSummaryValue;
use bbb_server_types::geometry::Aabb;
use uuid::Uuid;

pub(crate) fn apply_target_effects(
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    effects: Vec<super::TargetEffect>,
    detached: Option<&Entity>,
) {
    for effect in effects {
        match effect {
            super::TargetEffect::SetMobTarget { entity_id, target } => {
                set_mob_target_with_body(store, level, entity_id, target, detached)
            }
            super::TargetEffect::ForgetAndRefreshAnger { entity_id } => {
                forget_and_refresh_anger(store, level, entity_id)
            }
        }
    }
}
pub(crate) fn set_mob_target(
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    id: i32,
    target: Option<TargetRef>,
) {
    set_mob_target_with_body(store, level, id, target, None)
}
fn set_mob_target_with_body(
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    id: i32,
    target: Option<TargetRef>,
    detached: Option<&Entity>,
) {
    if target.is_none() {
        clear_mob_target(store, id);
        return;
    }
    let Some(mut body) = store.detach(id) else {
        return;
    };
    set_body_target_with_source(&mut body, store, level, target, detached);
    store.attach(body);
}
pub(crate) fn clear_mob_target(store: &mut EntityStore, id: i32) {
    let Some(kind) = store.get(id).map(Entity::entity_type_id) else {
        return;
    };
    let Some(runtime) = store.mob_ai_mut(id) else {
        return;
    };
    let mut writes = super::SetTargetWrites::default();
    let (state, random) = runtime.target_state_and_random();
    super::set_target_overrides(
        kind,
        0,
        false,
        false,
        random,
        &mut state.counters,
        &mut writes,
    );
    if let Some(time) = writes.enderman_target_change_time {
        state.target_change_time = time;
    }
    state.target = None;
    if let Some(body) = store.get_mut(id) {
        super::apply_set_target_writes(body, writes);
    }
}
pub(crate) fn set_body_target(
    body: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    target: Option<TargetRef>,
) -> bool {
    set_body_target_with_source(body, store, level, target, None)
}
fn set_body_target_with_source(
    body: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    target: Option<TargetRef>,
    detached: Option<&Entity>,
) -> bool {
    let sensing = Sensing::default();
    let (valid, had_target) = {
        let view = TargetView::new(body, store, level, &sensing).with_detached(detached);
        let candidate = target.and_then(|target| view.facts_of(target));
        if super::refuses_target(body.entity_type_id(), candidate.as_ref()) {
            return true;
        }
        let valid = candidate
            .filter(|candidate| super::conditions::is_valid_target(&view, candidate))
            .map(super::Target::target_ref);
        let had_target = store
            .mob_ai(body.id())
            .and_then(|runtime| runtime.target_ref())
            .and_then(|target| view.facts_of(target))
            .is_some_and(|target| super::conditions::is_valid_target(&view, &target));
        (valid, had_target)
    };
    let Some(runtime) = store.mob_ai_mut(body.id()) else {
        return false;
    };
    let mut writes = super::SetTargetWrites::default();
    let (state, random) = runtime.target_state_and_random();
    let tick = if state.load_target_pending {
        0
    } else {
        body.tick_count()
    };
    super::set_target_overrides(
        body.entity_type_id(),
        tick,
        had_target,
        target.is_some(),
        random,
        &mut state.counters,
        &mut writes,
    );
    if let Some(time) = writes.enderman_target_change_time {
        state.target_change_time = time;
    }
    state.target = valid;
    state.load_target_pending = false;
    super::apply_set_target_writes(body, writes);
    true
}

pub(crate) struct LoadedAngerTarget(Option<TargetRef>);
impl LoadedAngerTarget {
    pub(crate) fn apply(self, store: &mut EntityStore, id: i32) {
        let Some(body) = store.get(id) else {
            return;
        };
        let kind = body.entity_type_id();
        let tick = body.tick_count();
        let Some(runtime) = store.mob_ai_mut(id) else {
            return;
        };
        let mut writes = super::SetTargetWrites::default();
        let (state, random) = runtime.target_state_and_random();
        super::set_target_overrides(
            kind,
            tick,
            state.target.is_some(),
            self.0.is_some(),
            random,
            &mut state.counters,
            &mut writes,
        );
        if let Some(time) = writes.enderman_target_change_time {
            state.target_change_time = time;
        }
        state.target = self.0;
        state.load_target_pending = false;
        if let Some(body) = store.get_mut(id) {
            super::apply_set_target_writes(body, writes);
        }
    }
}
pub(crate) fn prepare_loaded_anger_target(
    body: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Option<LoadedAngerTarget> {
    let anger = super::anger::read(body)?;
    let sensing = Sensing::default();
    let view = TargetView::new(body, store, level, &sensing);
    let target = anger
        .target
        .and_then(|uuid| view.by_uuid(uuid))
        .filter(|target| super::conditions::is_valid_target(&view, target))
        .map(super::Target::target_ref);
    Some(LoadedAngerTarget(target))
}
pub(crate) fn body_has_live_target(
    body: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> bool {
    let sensing = Sensing::default();
    let view = TargetView::new(body, store, level, &sensing);
    store
        .mob_ai(body.id())
        .and_then(|runtime| runtime.target_ref())
        .and_then(|target| view.facts_of(target))
        .is_some_and(|target| target.alive() && super::conditions::is_valid_target(&view, &target))
}
pub(crate) fn before_ai_step(
    body: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
) -> bool {
    if store
        .mob_ai(body.id())
        .is_some_and(|runtime| runtime.target_state().load_target_pending)
    {
        let target = {
            let sensing = Sensing::default();
            let view = TargetView::new(body, store, level, &sensing);
            super::anger::read(body)
                .and_then(|anger| anger.target)
                .and_then(|uuid| view.by_uuid(uuid))
                .map(super::Target::target_ref)
        };
        if !set_body_target(body, store, level, target) {
            return false;
        }
        if let Some(runtime) = store.mob_ai_mut(body.id()) {
            runtime.target_state_mut().load_target_pending = false;
        }
    }
    if body.entity_type_id() == VANILLA_ENTITY_TYPE_ENDERMAN_ID {
        return update_body_anger(body, store, level, true);
    }
    true
}
pub(crate) fn after_ai_step(
    body: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
) -> bool {
    if matches!(
        body.entity_type_id(),
        VANILLA_ENTITY_TYPE_WOLF_ID | VANILLA_ENTITY_TYPE_IRON_GOLEM_ID
    ) {
        update_body_anger(body, store, level, true)
    } else {
        true
    }
}
pub(crate) fn after_tick(
    body: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
) -> bool {
    if body.entity_type_id() == VANILLA_ENTITY_TYPE_POLAR_BEAR_ID {
        update_body_anger(body, store, level, true)
    } else {
        true
    }
}
fn update_body_anger(
    body: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    stay: bool,
) -> bool {
    let Some(mut anger) = super::anger::read(body) else {
        return true;
    };
    let answer = store.with_mob_ai_control(body.id(), body.id(), |runtime, _, store| {
        let view = TargetView::new(body, store, level, &runtime.sensing);
        let previous = runtime
            .target_state
            .target
            .and_then(|target| view.facts_of(target));
        let valid = previous.filter(|target| super::conditions::is_valid_target(&view, target));
        let stopped = anger.update(
            &view,
            previous.as_ref(),
            valid.as_ref(),
            stay,
            &runtime.random,
        );
        let mut writes = super::SetTargetWrites::default();
        if stopped {
            super::set_target_overrides(
                body.entity_type_id(),
                body.tick_count(),
                valid.is_some(),
                false,
                &runtime.random,
                &mut runtime.target_state.counters,
                &mut writes,
            );
            runtime.target_state.target = None;
            if let Some(time) = writes.enderman_target_change_time {
                runtime.target_state.target_change_time = time;
            }
        }
        (stopped, writes)
    });
    let Some((stopped, writes)) = answer else {
        return false;
    };
    if stopped {
        let tick = body.tick_count();
        if let Some(living) = body.living_mut() {
            living.set_last_hurt_by_mob(None, tick);
        }
        super::apply_set_target_writes(body, writes);
    }
    super::anger::write(body, anger);
    true
}
pub(crate) fn bee_custom_step(
    body: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
) -> bool {
    update_body_anger(body, store, level, false)
}
pub(crate) fn bee_rolling(
    body: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Option<bool> {
    if !matches!(body.data(), EntityData::GeneralLiving(data) if data.bee_retained().is_some()) {
        return Some(false);
    }
    let sensing = Sensing::default();
    let view = TargetView::new(body, store, level, &sensing);
    if !super::anger::read(body)?.is_angry(store.game_time()) || view.bee_has_stung() {
        return Some(false);
    }
    let target = store
        .mob_ai(body.id())?
        .target_ref()
        .and_then(|target| view.facts_of(target));
    Some(target.is_some_and(|target| {
        super::conditions::is_valid_target(&view, &target)
            && view.body().distance_to_facts_sqr(&target) < 4.0
    }))
}
fn forget_and_refresh_anger(store: &mut EntityStore, level: &LevelCtx<'_, '_>, id: i32) {
    let Some(mut anger) = stop_being_angry(store, level, id) else {
        return;
    };
    let time = store.game_time();
    if let Some(runtime) = store.mob_ai_mut(id) {
        anger.start_timer(runtime.target_state_and_random().1, time);
    }
    if let Some(body) = store.get_mut(id) {
        super::anger::write(body, anger);
    }
}
fn stop_being_angry(
    store: &mut EntityStore,
    _level: &LevelCtx<'_, '_>,
    id: i32,
) -> Option<NeutralAnger> {
    let mut anger = store.get(id).and_then(super::anger::read)?;
    let body = store.get_mut(id)?;
    let tick = body.tick_count();
    if let Some(living) = body.living_mut() {
        living.set_last_hurt_by_mob(None, tick);
    }
    anger.clear();
    clear_mob_target(store, id);
    if let Some(body) = store.get_mut(id) {
        super::anger::write(body, anger);
    }
    Some(anger)
}
impl EntityStore {
    pub(crate) fn forgive_dead_player(
        &mut self,
        level: &LevelCtx<'_, '_>,
        player: Uuid,
        block: BlockPos,
    ) {
        let area = Aabb {
            min_x: f64::from(block.x),
            min_y: f64::from(block.y),
            min_z: f64::from(block.z),
            max_x: f64::from(block.x) + 1.0,
            max_y: f64::from(block.y) + 1.0,
            max_z: f64::from(block.z) + 1.0,
        }
        .inflate(32.0, 10.0, 32.0);
        let angry: Vec<i32> = self
            .ids_in_box_sections(area)
            .into_iter()
            .filter(|id| {
                self.get(*id).is_some_and(|entity| {
                    entity.bounding_box().intersects(area)
                        && super::anger::read(entity)
                            .is_some_and(|anger| anger.target == Some(player))
                })
            })
            .collect();
        for entity_id in angry {
            stop_being_angry(self, level, entity_id);
        }
    }
}

#[cfg(test)]
mod alliance_tests;
#[cfg(test)]
mod cross_level_tests;
#[cfg(test)]
mod pose_tests;
