//! Villagers approach nearby village heroes and throw profession gifts.

use crate::entity::ai::mob::target::{conditions::TargetingConditions, TargetView};
use crate::entity::ai::{
    Activity, MemoryModuleType, MemoryPositionTracker, MemoryValue, MemoryWalkTarget,
};
use crate::entity::store::EntityBodyReadView;
use crate::entity::{Entity, EntityData, EntityStore};
use crate::world::simulation_level::LevelCtx;
use bbb_server_types::item_stack::RuntimeItemStack;

#[derive(Clone, Copy)]
pub(in crate::entity::ai::mob) struct HeroGiftState {
    cooldown: i32,
    started: Option<(i64, i32)>,
}

impl Default for HeroGiftState {
    // `GiveGiftToHero.timeUntilNextGift`.
    fn default() -> Self {
        Self {
            cooldown: 600,
            started: None,
        }
    }
}

pub(crate) fn tick(body: &Entity, store: &mut EntityStore, level: &mut LevelCtx<'_, '_>) {
    if !crate::entity::villager::is_villager(body) {
        return;
    }
    let Some(runtime) = store.mob_ai(body.id()) else {
        return;
    };
    let Some(brain) = runtime.brain.as_ref() else {
        return;
    };
    let mut state = runtime.villager_hero_gift;
    let eligible = [Activity::WORK, Activity::MEET, Activity::IDLE]
        .into_iter()
        .any(|activity| brain.is_active(activity));
    let range = body.living().map_or(0.0, |living| {
        living.attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID)
    });
    let targets = TargetView::new(body, store, level, &runtime.sensing);
    let conditions = TargetingConditions::for_non_combat().range(range);
    let old_target = state.started.map(|(_, target)| target);
    let nearest = match brain.memory(MemoryModuleType::NearestVisiblePlayer) {
        Some(MemoryValue::EntityRef(id)) => Some(*id),
        _ => None,
    };
    // `VillagerGoalPackages.getWorkPackage`:
    // an unreachable hero must not hold the villager away from its daily work.
    let timed_out = state
        .started
        .is_some_and(|(started, _)| store.game_time() - started > 100);
    let target = (eligible && !timed_out)
        .then_some(nearest)
        .flatten()
        .and_then(|id| targets.facts(id))
        .filter(|target| targets.body().distance_to_facts_sqr(target) < range * range)
        .filter(|target| conditions.test(&targets, target, |_| true))
        .filter(|target| {
            store.player(target.entity_id()).is_some_and(|player| {
                player
                    .active_effect("minecraft:hero_of_the_village")
                    .is_some()
                    && !player.is_spectator()
                    && player.is_alive()
            })
        });
    let Some(target) = target else {
        if state.started.take().is_some() {
            clear_target(store, body.id(), old_target);
            state.cooldown = 600 + level.level_random().next_int(6001);
        }
        store.mob_ai_mut(body.id()).unwrap().villager_hero_gift = state;
        return;
    };
    if state.cooldown > 0 {
        state.cooldown -= 1;
        store.mob_ai_mut(body.id()).unwrap().villager_hero_gift = state;
        return;
    }
    let target_id = target.entity_id();
    let target_pos = target.position();
    let target_block = target.block_position();
    if old_target.is_some_and(|id| id != target_id) {
        clear_target(store, body.id(), old_target);
    }
    let at = body.block_position();
    // `GiveGiftToHero.isWithinThrowingDistance`.
    let close = (f64::from(at.x) - f64::from(target_block.x)).powi(2)
        + (f64::from(at.y) - f64::from(target_block.y)).powi(2)
        + (f64::from(at.z) - f64::from(target_block.z)).powi(2)
        < 25.0;
    let started = state.started.get_or_insert((store.game_time(), target_id));
    started.1 = target_id;
    let started = started.0;
    // `GiveGiftToHero.tick`. The existing brain
    // sinks own looking and walking; gift memories are refreshed after its choices.
    let runtime = store.mob_ai_mut(body.id()).unwrap();
    runtime.set_brain_memory(
        MemoryModuleType::InteractionTarget,
        MemoryValue::EntityRef(target_id),
    );
    runtime.set_brain_memory(
        MemoryModuleType::LookTarget,
        MemoryValue::EntityTracker(target_id),
    );
    if !close {
        runtime.set_brain_memory(
            MemoryModuleType::WalkTarget,
            MemoryValue::WalkTarget(MemoryWalkTarget {
                target: MemoryPositionTracker::Entity {
                    entity_id: target_id,
                    last_seen: target_block,
                },
                speed_modifier: 0.5,
                // The sink accepts equality; enter the strict throwing radius.
                close_enough_dist: 4,
            }),
        );
    }
    if close && store.game_time() - started > 20 {
        let drops = level.gift_loot_drops(
            EntityBodyReadView::new(store, Some(body), None),
            body,
            &gift_table(body),
        );
        for stack in drops {
            let stack = RuntimeItemStack::from_summary(stack);
            crate::entity::villager_life::throw_item(body, store, target_pos, stack);
        }
        clear_target(store, body.id(), Some(target_id));
        state.started = None;
        // `GiveGiftToHero.calculateTimeUntilNextGift`.
        state.cooldown = 600 + level.level_random().next_int(6001);
    }
    store.mob_ai_mut(body.id()).unwrap().villager_hero_gift = state;
}

fn clear_target(store: &mut EntityStore, body_id: i32, target: Option<i32>) {
    let Some(runtime) = store.mob_ai_mut(body_id) else {
        return;
    };
    for memory in [
        MemoryModuleType::InteractionTarget,
        MemoryModuleType::LookTarget,
        MemoryModuleType::WalkTarget,
    ] {
        let owned = match runtime.brain_memory(memory) {
            Some(MemoryValue::EntityRef(id) | MemoryValue::EntityTracker(id)) => {
                Some(*id) == target
            }
            Some(MemoryValue::WalkTarget(MemoryWalkTarget {
                target: MemoryPositionTracker::Entity { entity_id, .. },
                ..
            })) => Some(*entity_id) == target,
            _ => false,
        };
        if owned {
            runtime.erase_brain_memory(memory);
        }
    }
}

// `GiveGiftToHero.getLootTableToThrow`.
fn gift_table(body: &Entity) -> String {
    let profession = match body.data() {
        EntityData::GeneralLiving(data) if data.is_baby() => "baby",
        EntityData::GeneralLiving(data) => data
            .villager_profession_and_level()
            .map(|(profession, _)| profession)
            .and_then(|profession| profession.strip_prefix("minecraft:"))
            .filter(|profession| {
                super::PROFESSIONS
                    .iter()
                    .any(|known| known.strip_prefix("minecraft:") == Some(*profession))
                    && !matches!(*profession, "none" | "nitwit")
            })
            .unwrap_or("unemployed"),
        _ => "unemployed",
    };
    format!("minecraft:gameplay/hero_of_the_village/{profession}_gift")
}
