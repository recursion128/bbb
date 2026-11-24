//! Adult piglin retaliation and nearby anger propagation.

use super::{
    anger_uuid, attack_conditions, baby, can_be_hunted, can_hunt, nearest_visible_player,
    set_anger_target,
};
use crate::entity::ai::mob::target::{TargetRef, TargetView};
use crate::entity::{Entity, EntityStore};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_HOGLIN_ID, VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
    VANILLA_ENTITY_TYPE_PIGLIN_ID,
};

// PiglinAi.java:599-611; BehaviorUtils.java:124-134.
pub(super) fn was_hurt(
    body: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    attacker: TargetRef,
) {
    let current = store
        .mob_ai(body.id())
        .and_then(|runtime| runtime.target_ref());
    let (target, universal) = {
        let sensing = Default::default();
        let view = TargetView::new(body, store, level, &sensing)
            .with_detached(level.detached_entity(attacker.entity_id));
        let Some(threat) = view.facts_of(attacker).filter(|target| target.is_local()) else {
            return;
        };
        let mut conditions = attack_conditions(body).ignore_line_of_sight();
        if current == Some(attacker) {
            conditions = conditions.ignore_invisibility_testing();
        }
        if !conditions.test(&view, &threat, |_| true) {
            return;
        }
        let own = view.body();
        if current
            .and_then(|target| view.facts_of(target))
            .is_some_and(|target| {
                own.distance_to_facts_sqr(&threat) > own.distance_to_facts_sqr(&target) + 16.0
            })
        {
            return;
        }
        let universal = threat.is_player() && view.universal_anger();
        let target = if universal {
            nearest_visible_player(&view, current, |_| true).unwrap_or(attacker)
        } else {
            attacker
        };
        (target, universal)
    };
    if set_anger_target(body, store, level, target) {
        if universal {
            broadcast_universal_anger(body, store, level);
        } else {
            broadcast_anger_target(body, store, level, attacker);
        }
    }
}

// NearestLivingEntitySensor.java:17-24; PiglinAi.java:859-869.
fn nearby_adults(body: &Entity, store: &EntityStore) -> Vec<TargetRef> {
    let Some(living) = body.living() else {
        return Vec::new();
    };
    let range = living.attribute_value(VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID);
    let bounds = body.bounding_box().inflate(range, range, range);
    store
        .ids_in_box_sections(bounds)
        .into_iter()
        .filter_map(|id| {
            let peer = store.get(id)?;
            (id != body.id()
                && matches!(
                    peer.entity_type_id(),
                    VANILLA_ENTITY_TYPE_PIGLIN_ID | VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID
                )
                && !baby(peer)
                && peer.is_alive_with_health()
                && peer.bounding_box().intersects(bounds))
            .then_some(TargetRef {
                entity_id: id,
                uuid: peer.uuid(),
            })
        })
        .collect()
}

// PiglinAi.java:675-681; PiglinAi.java:712-722.
pub(super) fn broadcast_anger_target(
    body: &Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    target: TargetRef,
) {
    let huntable_hoglin = store
        .get(target.entity_id)
        .or_else(|| level.detached_entity(target.entity_id))
        .filter(|entity| entity.uuid() == target.uuid)
        .filter(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_HOGLIN_ID)
        .map(can_be_hunted);
    if huntable_hoglin == Some(false) {
        return;
    }
    for peer in nearby_adults(body, store) {
        store.with_entity_mut(peer.entity_id, |body, store| {
            if body.uuid() != peer.uuid || (huntable_hoglin.is_some() && !can_hunt(body)) {
                return;
            }
            let chosen = {
                let sensing = Default::default();
                let view = TargetView::new(body, store, level, &sensing)
                    .with_detached(level.detached_entity(target.entity_id));
                let Some(new_target) = view.facts_of(target).filter(|target| target.is_local())
                else {
                    return;
                };
                if anger_uuid(body)
                    .and_then(|uuid| view.by_uuid(uuid))
                    .filter(|current| current.is_local())
                    .is_some_and(|current| {
                        current.target_ref() == target
                            || view.body().distance_to_facts_sqr(&current)
                                < view.body().distance_to_facts_sqr(&new_target)
                    })
                {
                    return;
                }
                new_target.target_ref()
            };
            set_anger_target(body, store, level, chosen);
        });
    }
}

// PiglinAi.java:683-685.
fn broadcast_universal_anger(body: &Entity, store: &mut EntityStore, level: &LevelCtx<'_, '_>) {
    for peer in nearby_adults(body, store) {
        store.with_entity_mut(peer.entity_id, |body, store| {
            if body.uuid() != peer.uuid {
                return;
            }
            let target = {
                let sensing = Default::default();
                let view = TargetView::new(body, store, level, &sensing);
                let current = store
                    .mob_ai(body.id())
                    .and_then(|runtime| runtime.target_ref());
                nearest_visible_player(&view, current, |_| true)
            };
            if let Some(target) = target {
                set_anger_target(body, store, level, target);
            }
        });
    }
}
