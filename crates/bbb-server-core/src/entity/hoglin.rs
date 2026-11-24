//! Hoglin repellents, hurt retreat, and the shared hoglin/zoglin throw impulse.

use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_HOGLIN_ID, VANILLA_ENTITY_TYPE_PIGLIN_ID};
use bbb_protocol::packets::{BlockPos, Vec3d};

use crate::entity::ai::mob::target::conditions::TargetingConditions;
use crate::entity::ai::mob::target::{retaliation, TargetRef, TargetView};
use crate::entity::{Entity, EntityData, EntityStore, PendingEntitySound};
use crate::world::simulation_level::LevelCtx;
use bbb_server_types::geometry::Vec3Ops;

use crate::random::GameRandom;

// HoglinBase.java:38-53.
pub(crate) fn throw_impulse(
    from: Vec3d,
    to: Vec3d,
    knockback: f64,
    resistance: f64,
    random: &GameRandom,
) -> Option<Vec3d> {
    let power = knockback - resistance;
    if power <= 0.0 {
        return None;
    }
    let horizontal = Vec3d {
        x: to.x - from.x,
        y: 0.0,
        z: to.z - from.z,
    }
    .normalize()
    .scale(power * f64::from(random.next_float() * 0.5 + 0.2));
    // Vec3.java:237-243: the angle is in radians.
    let (sin, cos) = ((random.next_int(21) - 10) as f32).sin_cos();
    Some(Vec3d {
        x: horizontal.x * f64::from(cos) + horizontal.z * f64::from(sin),
        y: power * f64::from(random.next_float()) * 0.5,
        z: horizontal.z * f64::from(cos) - horizontal.x * f64::from(sin),
    })
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct HoglinState {
    pacified_ticks: u16,
    scan_delay: u8,
    nearest_repellent: Option<BlockPos>,
    avoid_target: Option<TargetRef>,
    retreat_ticks: u16,
    outnumbered: bool,
    nearest_adult_piglin: Option<TargetRef>,
}

pub(crate) fn is_pacified(entity: &Entity) -> bool {
    matches!(entity.data(), EntityData::GeneralLiving(data)
        if data.hoglin_state().is_some_and(|state| state.pacified_ticks > 0))
}

pub(crate) fn nearest_repellent(entity: &Entity) -> Option<BlockPos> {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    data.hoglin_state()?.nearest_repellent
}

pub(crate) fn retreat_target(entity: &Entity) -> Option<TargetRef> {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    data.hoglin_state()?.avoid_target
}

pub(crate) fn nearest_adult_piglin(entity: &Entity) -> Option<TargetRef> {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    data.hoglin_state()?.nearest_adult_piglin
}

// HoglinAi.java:120-129.
pub(crate) fn is_fleeing(entity: &Entity, attack_target: Option<TargetRef>) -> bool {
    retreat_target(entity).is_some() && attack_target.is_none()
}

fn attack_target(entity: &Entity, store: &EntityStore) -> Option<TargetRef> {
    store
        .mob_ai(entity.id())
        .and_then(|runtime| runtime.target_ref())
}

// HoglinAi.java:227-247.
pub(crate) fn ambient_sound(
    entity: &Entity,
    store: &EntityStore,
    converting: bool,
) -> Option<&'static str> {
    let target = store.mob_ai(entity.id())?.target_ref();
    Some(if is_fleeing(entity, target) || converting {
        "minecraft:entity.hoglin.retreat"
    } else if target.is_some() {
        "minecraft:entity.hoglin.angry"
    } else if nearest_repellent(entity).is_some() {
        "minecraft:entity.hoglin.retreat"
    } else {
        "minecraft:entity.hoglin.ambient"
    })
}

/// The activity-change sounds of `HoglinAi.updateActivity` (`HoglinAi.java:120-127,231-239`)
/// and `Zoglin.updateActivity` (`Zoglin.java:214-223`); avoid entry plays in [`install_retreat`].
pub(crate) fn update_activity_sound(
    entity: &Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
) {
    use crate::entity::ai::mob::CombatActivity;
    let zoglin =
        entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOGLIN_ID;
    let EntityData::GeneralLiving(data) = entity.data() else {
        return;
    };
    if (!zoglin && data.hoglin_state().is_none())
        || data.is_no_ai()
        || !entity.is_alive_with_health()
    {
        return;
    }
    let target = attack_target(entity, store);
    let now = if target.is_some() {
        CombatActivity::Fight
    } else if !zoglin && retreat_target(entity).is_some() {
        CombatActivity::Avoid
    } else {
        CombatActivity::Idle
    };
    let Some(previous) = store
        .mob_ai_mut(entity.id())
        .map(|runtime| runtime.swap_combat_activity(now))
    else {
        return;
    };
    if previous == now || entity.is_silent() {
        return;
    }
    let converting = || {
        data.zombification().is_some_and(|state| {
            state.is_converting(
                data.is_no_ai(),
                level.piglins_zombify(entity.position()) == Some(true),
            )
        })
    };
    let sound = match now {
        CombatActivity::Fight if zoglin => "minecraft:entity.zoglin.angry",
        _ if zoglin => return,
        CombatActivity::Avoid => return,
        CombatActivity::Fight if !converting() => "minecraft:entity.hoglin.angry",
        CombatActivity::Idle if !converting() && nearest_repellent(entity).is_none() => {
            "minecraft:entity.hoglin.ambient"
        }
        _ => "minecraft:entity.hoglin.retreat",
    };
    let baby = data.is_baby();
    let pitch = crate::entity::living::voice_pitch(store, baby);
    store.play_entity_sound(PendingEntitySound {
        position: entity.position(),
        resource_id: sound.to_owned(),
        source: crate::entity::living::living_sound_source(entity),
        volume: 1.0,
        pitch,
    });
}

// HoglinSpecificSensor.java:31-56; NearestLivingEntitySensor.java:17-24.
fn visit_visible_adults(
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    mut visit: impl FnMut(TargetRef, i32, Vec3d),
) {
    let Some(living) = entity.living() else {
        return;
    };
    let range = living.attribute_value(VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID);
    let bounds = entity.bounding_box().inflate(range, range, range);
    let conditions = TargetingConditions::for_non_combat().range(range);
    let current_attack = attack_target(entity, store);
    let sensing = Default::default();
    let view = TargetView::new(entity, store, level, &sensing);
    for target in view.local_stored() {
        let kind = target.entity_type_id();
        if !matches!(
            kind,
            VANILLA_ENTITY_TYPE_HOGLIN_ID | VANILLA_ENTITY_TYPE_PIGLIN_ID
        ) || target.baby()
            || !target.alive()
            || !target.bounding_box().intersects(bounds)
        {
            continue;
        }
        let conditions = if current_attack == Some(target.target_ref()) {
            conditions.ignore_invisibility_testing()
        } else {
            conditions
        };
        if conditions.test(&view, &target, |_| true) {
            visit(target.target_ref(), kind, target.position());
        }
    }
}

pub(crate) fn visible_adult_hoglins(
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Vec<TargetRef> {
    let mut herd = Vec::new();
    visit_visible_adults(entity, store, level, |target, kind, _| {
        if kind == VANILLA_ENTITY_TYPE_HOGLIN_ID {
            herd.push(target);
        }
    });
    herd
}

pub(crate) fn tick_ai(entity: &mut Entity, store: &EntityStore, level: &LevelCtx<'_, '_>) {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return;
    };
    let Some(state) = data.hoglin_state() else {
        return;
    };
    let baby = data.is_baby();
    let fleeing = is_fleeing(entity, attack_target(entity, store));
    let balance = (state.scan_delay == 0).then(|| {
        let mut piglins = 0_usize;
        let mut hoglins = 1_usize;
        let mut nearest = None;
        visit_visible_adults(entity, store, level, |target, kind, position| {
            if kind == VANILLA_ENTITY_TYPE_PIGLIN_ID {
                piglins += 1;
                let distance = entity.position().distance_to_sqr(position);
                if nearest.is_none_or(|(best, _)| distance < best) {
                    nearest = Some((distance, target));
                }
            } else {
                hoglins += 1;
            }
        });
        // HoglinAi.java:174-181.
        (piglins > hoglins, nearest.map(|(_, target)| target))
    });
    let center = entity.block_position();
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let Some(state) = data.hoglin_state_mut() else {
        return;
    };
    state.pacified_ticks = state.pacified_ticks.saturating_sub(1);
    state.retreat_ticks = state.retreat_ticks.saturating_sub(1);
    if let Some((outnumbered, nearest)) = balance {
        state.outnumbered = outnumbered;
        // HoglinSpecificSensor.java:40-53.
        state.nearest_adult_piglin = nearest;
    }
    // HoglinAi.java:170-181: only adults can end avoidance before its expiry.
    if state.retreat_ticks == 0 || (!baby && fleeing && !state.outnumbered) {
        state.avoid_target = None;
        state.retreat_ticks = 0;
    }
    if state.scan_delay == 0 {
        // Sensor.java:14: the default sensing interval is 20 ticks.
        state.scan_delay = 19;
        let mut nearest = None;
        // HoglinSpecificSensor.java:59-61; HoglinAi.java:42-43.
        for x in -8_i32..=8 {
            for y in -4_i32..=4 {
                for z in -8_i32..=8 {
                    let distance = x.abs() + y.abs() + z.abs();
                    if nearest.is_some_and(|(best, _)| distance >= best) {
                        continue;
                    }
                    let pos = BlockPos {
                        x: center.x + x,
                        y: center.y + y,
                        z: center.z + z,
                    };
                    if level
                        .block_state_in_tag(level.block_state(pos), "minecraft:hoglin_repellents")
                    {
                        nearest = Some((distance, pos));
                    }
                }
            }
        }
        state.nearest_repellent = nearest.map(|(_, pos)| pos);
    } else {
        state.scan_delay -= 1;
    }
    // HoglinAi.java:50; BecomePassiveIfMemoryPresent.java:8-18: an existing expiry is not refreshed.
    if state.pacified_ticks == 0 && state.nearest_repellent.is_some() {
        state.pacified_ticks = 200;
    }
}

// Hoglin.java:115-121; HoglinAi.java:184-192; HoglinAi.java:147-159.
pub(crate) fn was_hurt(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    attacker: Option<TargetRef>,
) {
    let Some(attacker) = attacker else {
        return;
    };
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let baby = data.is_baby();
    let Some(state) = data.hoglin_state_mut() else {
        return;
    };
    state.pacified_ticks = 0;
    if !baby {
        return;
    }
    retreat_from_nearest(entity, store, level, attacker);
}

// HoglinAi.java:132-145.
pub(crate) fn on_hit_target(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    target_id: i32,
) {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return;
    };
    let Some(state) = data.hoglin_state() else {
        return;
    };
    if data.is_baby() {
        return;
    }
    let outnumbered = state.outnumbered;
    let target = {
        let sensing = Default::default();
        let view = TargetView::new(entity, store, level, &sensing)
            .with_detached(level.detached_entity(target_id));
        view.facts(target_id)
            .map(|target| (target.target_ref(), target.entity_type_id()))
    };
    let Some((target, kind)) = target else {
        return;
    };
    if kind == VANILLA_ENTITY_TYPE_PIGLIN_ID && outnumbered {
        install_retreat(entity, store, target);
        for peer in visible_adult_hoglins(entity, store, level) {
            store.with_entity_mut(peer.entity_id, |body, store| {
                if body.uuid() == peer.uuid {
                    retreat_from_nearest(body, store, level, target);
                }
            });
        }
    } else {
        retaliation::alert_hoglins(entity, store, level, target);
    }
}

fn retreat_from_nearest(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    attacker: TargetRef,
) {
    let previous_avoid = retreat_target(entity);
    let nearest = {
        let sensing = Default::default();
        let view = TargetView::new(entity, store, level, &sensing)
            .with_detached(level.detached_entity(attacker.entity_id));
        let previous_attack = attack_target(entity, store);
        let origin = entity.position();
        [Some(attacker), previous_avoid, previous_attack]
            .into_iter()
            .flatten()
            .filter_map(|target| view.facts_of(target))
            .min_by(|a, b| {
                origin
                    .distance_to_sqr(a.position())
                    .total_cmp(&origin.distance_to_sqr(b.position()))
            })
            .map(|target| target.target_ref())
    };
    let Some(nearest) = nearest else {
        return;
    };
    install_retreat(entity, store, nearest);
}

fn install_retreat(entity: &mut Entity, store: &mut EntityStore, target: TargetRef) {
    let was_fleeing = is_fleeing(entity, attack_target(entity, store));
    // HoglinAi.java:44; TimeUtil.java:13-15.
    let duration = store.random().next_int_between_inclusive(100, 400) as u16;
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let Some(state) = data.hoglin_state_mut() else {
        return;
    };
    state.avoid_target = Some(target);
    state.retreat_ticks = duration;
    if let Some(runtime) = store.mob_ai_mut(entity.id()) {
        let target = runtime.target_state_mut();
        target.target = None;
        target.retaliation_ticks = None;
        runtime.stop_navigation();
    }
    // HoglinAi.java:125-126; HoglinAi.java:231-237; LivingEntity.java:2280-2288.
    if !was_fleeing && entity.is_alive_with_health() && !entity.is_silent() {
        let baby = matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_baby());
        let pitch = crate::entity::living::voice_pitch(store, baby);
        store.play_entity_sound(PendingEntitySound {
            position: entity.position(),
            resource_id: "minecraft:entity.hoglin.retreat".to_owned(),
            source: crate::entity::living::living_sound_source(entity),
            volume: 1.0,
            pitch,
        });
    }
}
