//! Hurt avoidance and visible adult piglin retreat coordination.

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_HOGLIN_ID, VANILLA_ENTITY_TYPE_PIGLIN_ID, VANILLA_ENTITY_TYPE_ZOGLIN_ID,
    VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
};
use bbb_server_types::geometry::Vec3Ops;

use crate::entity::ai::mob::target::{TargetRef, TargetView};
use crate::entity::ai::MemoryModuleType;
use crate::entity::{Entity, EntityStore, PendingEntitySound};
use crate::world::simulation_level::LevelCtx;

use super::{
    attack_conditions, baby, dont_hunt, erase, is_admiring, is_avoiding, visit_visible_adults,
};

// PiglinAi.java:296-303; CopyMemoryWithExpiry.java:13-19.
pub(super) fn sense(
    body: &Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    nemesis: Option<TargetRef>,
) {
    let Some(runtime) = store.mob_ai(body.id()) else {
        return;
    };
    let Some(piglin) = runtime.piglin() else {
        return;
    };
    if piglin.avoid_target.is_some() {
        return;
    }
    let active = runtime.target_ref().is_none() && !is_admiring(body);
    let target = if baby(body) { nemesis } else { None }.or_else(|| {
        let target = piglin.nearest_zombified?;
        let sensing = Default::default();
        TargetView::new(body, store, level, &sensing)
            .with_detached(level.detached_entity(target.entity_id))
            .facts_of(target)
            // PiglinAi.java:501-508.
            .filter(|target| body.position().distance_to_sqr(target.position()) < 36.0)
            .map(|target| target.target_ref())
    });
    let Some(target) = target else {
        return;
    };
    // PiglinAi.java:108-109.
    let duration = store.random().next_int_between_inclusive(100, 140) as u16;
    if active {
        play_entry_sound(body, store, level, target);
    }
    let Some(runtime) = store.mob_ai_mut(body.id()) else {
        return;
    };
    let Some(piglin) = runtime.piglin_mut() else {
        return;
    };
    write_avoid(piglin, target, duration);
    if active {
        runtime.stop_navigation();
    }
}

// PiglinAi.java:750-771.
pub(super) fn tick(body: &mut Entity, store: &mut EntityStore, level: &LevelCtx<'_, '_>) {
    let Some(runtime) = store.mob_ai(body.id()) else {
        return;
    };
    let Some(piglin) = runtime.piglin() else {
        return;
    };
    let avoiding = is_avoiding(body, store) && !is_admiring(body);
    let recovered = piglin.avoid_target.is_some_and(|target| {
        let sensing = Default::default();
        TargetView::new(body, store, level, &sensing)
            .with_detached(level.detached_entity(target.entity_id))
            .facts_of(target)
            .filter(|target| target.is_local() && target.alive())
            .is_none_or(|target| match target.entity_type_id() {
                VANILLA_ENTITY_TYPE_HOGLIN_ID => !piglin.outnumbered,
                VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID | VANILLA_ENTITY_TYPE_ZOGLIN_ID => {
                    Some(target.target_ref()) != piglin.nearest_zombified
                }
                _ => false,
            })
    });
    let Some(runtime) = store.mob_ai_mut(body.id()) else {
        return;
    };
    let Some(piglin) = runtime.piglin_mut() else {
        return;
    };
    piglin.retreat_ticks = piglin.retreat_ticks.saturating_sub(1);
    if piglin.avoid_target.is_some() && (piglin.retreat_ticks == 0 || (avoiding && recovered)) {
        piglin.avoid_target = None;
        piglin.retreat_ticks = 0;
        if avoiding {
            runtime.stop_navigation();
        }
    }
}

// PiglinAi.java:580-595.
pub(super) fn was_hurt(
    body: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    attacker: TargetRef,
) -> bool {
    let Some(runtime) = store.mob_ai(body.id()) else {
        return false;
    };
    let Some(piglin) = runtime.piglin() else {
        return false;
    };
    let current = runtime.target_ref();
    let old_avoid = piglin.avoid_target;
    let outnumbered = piglin.outnumbered;
    let (kind, clear_old, attackable) = {
        let sensing = Default::default();
        let view = TargetView::new(body, store, level, &sensing)
            .with_detached(level.detached_entity(attacker.entity_id));
        let Some(threat) = view.facts_of(attacker) else {
            return false;
        };
        let kind = threat.entity_type_id();
        let clear_old = old_avoid
            .and_then(|target| view.facts_of(target))
            .is_some_and(|target| target.entity_type_id() != kind);
        let mut conditions = attack_conditions(body).ignore_line_of_sight();
        if current == Some(attacker) {
            conditions = conditions.ignore_invisibility_testing();
        }
        (kind, clear_old, conditions.test(&view, &threat, |_| true))
    };
    if baby(body) {
        install_avoid(body, store, level, attacker, 100);
        if attackable {
            super::combat::broadcast_anger_target(body, store, level, attacker);
        }
        return true;
    }
    if kind == VANILLA_ENTITY_TYPE_HOGLIN_ID && outnumbered {
        let mut peers = Vec::new();
        // PiglinAi.java:734-747: brutes count toward strength but do not retreat.
        visit_visible_adults(body, store, level, |peer, _| {
            if peer.entity_type_id() == VANILLA_ENTITY_TYPE_PIGLIN_ID {
                peers.push(TargetRef {
                    entity_id: peer.id(),
                    uuid: peer.uuid(),
                });
            }
        });
        install_retreat(body, store, level, attacker);
        for peer in peers {
            store.with_entity_mut(peer.entity_id, |body, store| {
                if body.uuid() == peer.uuid {
                    retreat_from_nearest(body, store, level, attacker);
                }
            });
        }
        return true;
    }
    if clear_old {
        if let Some(piglin) = store
            .mob_ai_mut(body.id())
            .and_then(|runtime| runtime.piglin_mut())
        {
            piglin.avoid_target = None;
            piglin.retreat_ticks = 0;
        }
    }
    is_avoiding(body, store)
}

fn retreat_from_nearest(
    body: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    attacker: TargetRef,
) {
    let Some(runtime) = store.mob_ai(body.id()) else {
        return;
    };
    let Some(piglin) = runtime.piglin() else {
        return;
    };
    let nearest = {
        let sensing = Default::default();
        let view = TargetView::new(body, store, level, &sensing)
            .with_detached(level.detached_entity(attacker.entity_id));
        [Some(attacker), piglin.avoid_target, runtime.target_ref()]
            .into_iter()
            .flatten()
            .filter_map(|target| view.facts_of(target))
            .min_by(|a, b| {
                body.position()
                    .distance_to_sqr(a.position())
                    .total_cmp(&body.position().distance_to_sqr(b.position()))
            })
            .map(|target| target.target_ref())
    };
    if let Some(target) = nearest {
        install_retreat(body, store, level, target);
    }
}

// PiglinAi.java:774-779.
fn install_retreat(
    body: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    target: TargetRef,
) {
    let duration = store.random().next_int_between_inclusive(100, 400) as u16;
    erase(body, MemoryModuleType::AngryAt);
    install_avoid(body, store, level, target, duration);
    dont_hunt(body, store.random());
}

fn install_avoid(
    body: &Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    target: TargetRef,
    duration: u16,
) {
    play_entry_sound(body, store, level, target);
    let Some(runtime) = store.mob_ai_mut(body.id()) else {
        return;
    };
    let Some(piglin) = runtime.piglin_mut() else {
        return;
    };
    write_avoid(piglin, target, duration);
    let target = runtime.target_state_mut();
    target.target = None;
    target.retaliation_ticks = None;
    runtime.stop_navigation();
}

fn write_avoid(piglin: &mut super::PiglinRuntime, target: TargetRef, duration: u16) {
    piglin.avoid_target = Some(target);
    piglin.retreat_ticks = duration;
}

fn play_entry_sound(
    body: &Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    target: TargetRef,
) {
    // PiglinAi.java:624-625; PiglinAi.java:637-640.
    let play_sound = !is_avoiding(body, store)
        && !is_admiring(body)
        && body.is_alive_with_health()
        && !body.is_silent()
        && {
            let sensing = Default::default();
            TargetView::new(body, store, level, &sensing)
                .with_detached(level.detached_entity(target.entity_id))
                .facts_of(target)
                .is_some_and(|target| body.position().distance_to_sqr(target.position()) < 144.0)
        };
    if play_sound {
        let pitch = crate::entity::living::voice_pitch(store, baby(body));
        store.play_entity_sound(PendingEntitySound {
            position: body.position(),
            resource_id: "minecraft:entity.piglin.retreat".to_owned(),
            source: crate::entity::living::living_sound_source(body),
            volume: 1.0,
            pitch,
        });
    }
}
