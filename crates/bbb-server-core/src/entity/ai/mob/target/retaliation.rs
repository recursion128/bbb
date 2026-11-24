//! Hoglin and zoglin retaliation targets and their finite pursuit time.

use super::{conditions, TargetRef, TargetRuntime, TargetView};
use crate::entity::ai::mob::MobAiContext;
use crate::entity::{Entity, EntityStore, StoredEntityDamageSource};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_HOGLIN_ID, VANILLA_ENTITY_TYPE_PIGLIN_ID, VANILLA_ENTITY_TYPE_ZOGLIN_ID,
};

pub(crate) fn was_hurt(
    body: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    source: StoredEntityDamageSource<'_>,
    living_attacker: bool,
) {
    let hoglin = body.entity_type_id() == VANILLA_ENTITY_TYPE_HOGLIN_ID;
    if !(hoglin || body.entity_type_id() == VANILLA_ENTITY_TYPE_ZOGLIN_ID) || !living_attacker {
        return;
    }
    let Some(attacker) = source.causing_entity else {
        return;
    };
    let accepted = store.with_mob_ai_control(body.id(), body.id(), |runtime, _, store| {
        let view = TargetView::new(body, store, level, &runtime.sensing)
            .with_detached(level.detached_entity(attacker.entity_id()));
        let attacker = view.facts(attacker.entity_id())?;
        let own = view.body();
        if hoglin && (own.baby() || attacker.entity_type_id() == VANILLA_ENTITY_TYPE_HOGLIN_ID) {
            return None;
        }
        if hoglin
            && attacker.entity_type_id() == VANILLA_ENTITY_TYPE_PIGLIN_ID
            && crate::entity::hoglin::is_fleeing(body, runtime.target_state.target)
        {
            return None;
        }
        if !conditions::can_attack(view.peaceful(), &own, &attacker) {
            return None;
        }
        if runtime
            .target_state
            .target
            .and_then(|target| view.facts_of(target))
            .is_some_and(|current| {
                own.distance_to_facts_sqr(&attacker) > own.distance_to_facts_sqr(&current) + 16.0
            })
        {
            return None;
        }
        if hoglin {
            // Sensor.
            let range = body
                .living()
                .expect("hoglin living state")
                .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID);
            let mut conditions = conditions::TargetingConditions::for_combat().range(range);
            if runtime.target_state.target == Some(attacker.target_ref()) {
                conditions = conditions.ignore_invisibility_testing();
            }
            if !conditions.test(&view, &attacker, |_| true) {
                return None;
            }
        }
        let target = attacker.target_ref();
        set_attack_target(&mut runtime.target_state, target);
        Some(target)
    });
    if let Some(target) = accepted.flatten().filter(|_| hoglin) {
        alert_hoglins(body, store, level, target);
    }
}

pub(crate) fn alert_hoglins(
    body: &Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    target: TargetRef,
) {
    for peer in crate::entity::hoglin::visible_adult_hoglins(body, store, level) {
        store.with_mob_ai_control(peer.entity_id, peer.entity_id, |runtime, _, store| {
            let Some(peer_body) = store
                .get(peer.entity_id)
                .filter(|body| body.uuid() == peer.uuid)
            else {
                return;
            };
            if crate::entity::hoglin::is_pacified(peer_body) {
                return;
            }
            let view = TargetView::new(peer_body, store, level, &runtime.sensing)
                .with_detached(level.detached_entity(target.entity_id));
            let Some(new_target) = view.facts_of(target) else {
                return;
            };
            let own = view.body();
            let target = runtime
                .target_state
                .target
                .and_then(|current| view.facts_of(current))
                .filter(|current| {
                    own.distance_to_facts_sqr(current) < own.distance_to_facts_sqr(&new_target)
                })
                .map_or(target, |current| current.target_ref());
            set_attack_target(&mut runtime.target_state, target);
        });
    }
}

fn set_attack_target(state: &mut TargetRuntime, target: TargetRef) {
    state.target = Some(target);
    state.retaliation_ticks = Some(200);
}

pub(in crate::entity::ai::mob) fn retain_retaliation(
    state: &mut TargetRuntime,
    context: &mut MobAiContext,
    targets: &TargetView<'_, '_, '_>,
) -> bool {
    let Some(ticks) = state.retaliation_ticks.as_mut() else {
        return false;
    };
    if *ticks > 0
        && context.unchecked_target(targets).is_some_and(|target| {
            target.is_local()
                && conditions::can_attack(targets.peaceful(), &targets.body(), &target)
        })
    {
        *ticks -= 1;
        return true;
    }
    state.retaliation_ticks = None;
    context.set_target(None, targets);
    false
}
