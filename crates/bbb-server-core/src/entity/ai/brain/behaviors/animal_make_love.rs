//! `AnimalMakeLove`, which `FrogAi` registers
//! in IDLE at priority 0 beside the look target.

use super::super::effect::BrainWorldEffect;
use super::super::memory::{
    MemoryModuleType, MemoryPositionTracker, MemoryStatus, MemoryValue, MemoryWalkTarget,
};
use super::super::sensors::is_entity_targetable;
use super::super::{Behavior, BehaviorHooks, BrainTick, BrainTickContext};
use super::{containing_block_pos, entity_tracker};

/// `AnimalMakeLove.MIN_DURATION`/`MAX_DURATION` (`AnimalMakeLove.java:14-15`),
/// which the `super(entryCondition, 110)` constructor passes as both
/// (`:40`).
const DURATION: i32 = 110;

/// `AnimalMakeLove.BREED_RANGE` (`AnimalMakeLove.java:13`), the distance the
/// mating itself needs (`:77`).
const BREED_RANGE: f64 = 3.0;

/// `AnimalMakeLove.MIN_DURATION` (`:14`), the floor of the
/// `60 + body.getRandom().nextInt(50)` draw (`:56`).
const SPAWN_CHILD_MIN_DELAY: i32 = 60;

/// `MAX_DURATION - MIN_DURATION` (`:14-15`), the bound of the same draw.
const SPAWN_CHILD_DELAY_BOUND: i32 = 50;

/// `AnimalMakeLove` as
/// `new AnimalMakeLove(EntityType.FROG)` builds it.
///
/// The partner search is `findValidBreedPartner` over the body's
/// own visible-entity memory. The two reads it makes on the candidate beyond
/// that memory — `Animal.canMate` and
/// `PathfinderMob.isPanicking` — are answered by
/// the world from the projection its runtime took before this tick.
#[derive(Debug, Clone, Copy)]
pub struct AnimalMakeLove {
    /// `AnimalMakeLove.partnerType` (`:16`), the type every candidate must be
    /// (`:99`, `:106`).
    partner_type: &'static str,
    /// `AnimalMakeLove.speedModifier` (`:17`), the speed both walk targets are
    /// written with (`:55`, `:76`).
    speed_modifier: f32,
    /// `AnimalMakeLove.closeEnoughDistance` (`:18`), the close-enough distance
    /// of both walk targets (`:55`, `:76`).
    close_enough_distance: i32,
    /// `AnimalMakeLove.spawnChildAtTime` (`:20`), `0L` at construction.
    spawn_child_at_time: i64,
}

impl AnimalMakeLove {
    /// `AnimalMakeLove.hasBreedTargetOfRightType(body)` (`:97-100`): the memory
    /// holds a value and that value is the configured partner type.
    fn has_breed_target_of_right_type(&self, tick: &BrainTick<'_>) -> bool {
        tick.memory_entity(MemoryModuleType::BreedTarget)
            .is_some_and(|entity_id| {
                tick.world()
                    .entity_facts(entity_id)
                    .is_some_and(|facts| facts.entity_resource_id == self.partner_type)
            })
    }

    /// `AnimalMakeLove.findValidBreedPartner(body)` (`:102-108`) over
    /// `NearestVisibleLivingEntities.findClosest` (`:106`), whose filter is
    /// `entity.is(partnerType)`, `entity instanceof Animal`,
    /// `body.canMate(animal)` and `!animal.isPanicking()`, and whose own test
    /// is `Sensor.isEntityTargetable`.
    fn find_valid_breed_partner(&self, tick: &BrainTick<'_>) -> Option<i32> {
        let Some(MemoryValue::VisibleEntities(visible)) =
            tick.memory(MemoryModuleType::NearestVisibleLivingEntities)
        else {
            return None;
        };
        visible.iter().copied().find(|entity_id| {
            let facts = tick.world().entity_facts(*entity_id);
            facts.is_some_and(|facts| facts.entity_resource_id == self.partner_type)
                && self.can_mate(tick, *entity_id)
                && tick
                    .world()
                    .entity_breed_facts(*entity_id)
                    .is_some_and(|facts| !facts.panicking)
                && is_entity_targetable(tick, *entity_id)
        })
    }

    /// `Animal.canMate(partner)` as the body's own
    /// brain reads it: `partner != this`, the same class on both sides, and
    /// both animals in love. `getClass()` is each type's own class in 26.1, so
    /// the entity type stands in for it.
    fn can_mate(&self, tick: &BrainTick<'_>, partner_id: i32) -> bool {
        tick.world().body().entity_id != partner_id
            && tick.world().body().is_in_love()
            && tick
                .world()
                .entity_breed_facts(partner_id)
                .is_some_and(|facts| facts.same_class && facts.in_love)
    }

    /// `AnimalMakeLove`'s gaze lock (`:55`, `:76`).
    fn lock_gaze_and_walk_to_each_other(&self, tick: &mut BrainTick<'_>, partner_id: i32) {
        lock_gaze_and_walk_to_each_other(
            tick,
            partner_id,
            self.speed_modifier,
            self.close_enough_distance,
        );
    }
}

/// `BehaviorUtils.lockGazeAndWalkToEachOther(body, partner, speedModifier,
/// closeEnoughDistance)`.
///
/// Both brains take an entity-tracker look target and an entity-tracker walk
/// target over each other; the partner is another
/// mob, whose live brain this tick cannot borrow, so its two entries go
/// through the effect channel.
pub(super) fn lock_gaze_and_walk_to_each_other(
    tick: &mut BrainTick<'_>,
    partner_id: i32,
    speed_modifier: f32,
    close_enough_dist: i32,
) {
    let Some(partner_tracker) = entity_tracker(tick, partner_id) else {
        return;
    };
    let body_id = tick.world().body().entity_id;
    let body_tracker = MemoryPositionTracker::Entity {
        entity_id: body_id,
        last_seen: containing_block_pos(tick.world().body().position),
    };
    let walk_target = |target| MemoryWalkTarget {
        target,
        speed_modifier,
        close_enough_dist,
    };
    tick.set_memory_value(
        MemoryModuleType::LookTarget,
        MemoryValue::EntityTracker(partner_id),
    );
    queue_partner_memory(
        tick,
        partner_id,
        MemoryModuleType::LookTarget,
        MemoryValue::EntityTracker(body_id),
    );
    tick.set_memory_value(
        MemoryModuleType::WalkTarget,
        MemoryValue::WalkTarget(walk_target(partner_tracker)),
    );
    queue_partner_memory(
        tick,
        partner_id,
        MemoryModuleType::WalkTarget,
        MemoryValue::WalkTarget(walk_target(body_tracker)),
    );
}

/// One `Brain.setMemory` this tick owes another mob's brain
fn queue_partner_memory(
    tick: &mut BrainTick<'_>,
    entity_id: i32,
    memory_type: MemoryModuleType,
    value: MemoryValue,
) {
    tick.world_mut()
        .queue_effect(BrainWorldEffect::SetMobBrainMemory {
            entity_id,
            memory_type,
            value: Some(value),
        });
}

impl BehaviorHooks for AnimalMakeLove {
    // `AnimalMakeLove.checkExtraStartConditions`.
    fn extra_start_conditions(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
    ) -> bool {
        tick.world().body().is_in_love() && self.find_valid_breed_partner(tick).is_some()
    }

    // `AnimalMakeLove.start` (`:51-58`).
    fn start(&mut self, tick: &mut BrainTick<'_>, context: &mut BrainTickContext, timestamp: i64) {
        let Some(partner_id) = self.find_valid_breed_partner(tick) else {
            return;
        };
        let body_id = tick.world().body().entity_id;
        tick.set_memory_value(
            MemoryModuleType::BreedTarget,
            MemoryValue::EntityRef(partner_id),
        );
        queue_partner_memory(
            tick,
            partner_id,
            MemoryModuleType::BreedTarget,
            MemoryValue::EntityRef(body_id),
        );
        self.lock_gaze_and_walk_to_each_other(tick, partner_id);
        // `int duration = 60 + body.getRandom().nextInt(50)` (`:56`).
        self.spawn_child_at_time = timestamp
            + i64::from(
                SPAWN_CHILD_MIN_DELAY + context.body_random().next_int(SPAWN_CHILD_DELAY_BOUND),
            );
    }

    // `AnimalMakeLove.canStillUse` (`:60-72`), in the source's own term order:
    // a later term is never reached once an earlier one refused.
    fn can_still_use(
        &self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        timestamp: i64,
    ) -> bool {
        if !self.has_breed_target_of_right_type(tick) {
            return false;
        }
        let Some(partner_id) = tick.memory_entity(MemoryModuleType::BreedTarget) else {
            return false;
        };
        let partner_is_alive = tick
            .world()
            .entity_facts(partner_id)
            .is_some_and(|facts| facts.alive);
        if !partner_is_alive || !self.can_mate(tick, partner_id) {
            return false;
        }
        if !is_entity_targetable(tick, partner_id) {
            return false;
        }
        if timestamp > self.spawn_child_at_time
            || tick.has_memory_value(MemoryModuleType::IsPanicking)
        {
            return false;
        }
        tick.world()
            .entity_breed_facts(partner_id)
            .is_some_and(|facts| !facts.panicking)
    }

    // `AnimalMakeLove.tick` (`:74-84`).
    fn tick(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, timestamp: i64) {
        let Some(partner_id) = tick.memory_entity(MemoryModuleType::BreedTarget) else {
            return;
        };
        self.lock_gaze_and_walk_to_each_other(tick, partner_id);
        // `body.closerThan(partner, BREED_RANGE)` (`:77`).
        let in_range = tick
            .world()
            .entity_distance_to_sqr(partner_id)
            .is_some_and(|distance| distance < BREED_RANGE * BREED_RANGE);
        if !in_range || timestamp < self.spawn_child_at_time {
            return;
        }
        // `body.spawnChildFromBreeding(level, partner)` and the two erases that
        // follow it (`:79-81`).
        tick.world_mut()
            .queue_effect(BrainWorldEffect::SpawnChildFromBreeding { partner_id });
        tick.erase_memory(MemoryModuleType::BreedTarget);
        tick.world_mut()
            .queue_effect(BrainWorldEffect::SetMobBrainMemory {
                entity_id: partner_id,
                memory_type: MemoryModuleType::BreedTarget,
                value: None,
            });
    }

    // `AnimalMakeLove.stop` (`:86-91`); the partner's own `BREED_TARGET` is
    // left where it is, which is what the source does.
    fn stop(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, _timestamp: i64) {
        tick.erase_memory(MemoryModuleType::BreedTarget);
        tick.erase_memory(MemoryModuleType::WalkTarget);
        tick.erase_memory(MemoryModuleType::LookTarget);
        self.spawn_child_at_time = 0;
    }

    fn debug_string(&self) -> &'static str {
        "AnimalMakeLove"
    }
}

/// `new AnimalMakeLove(partnerType)`, which
/// `FrogAi` registers as `new AnimalMakeLove(EntityType.FROG)`
pub fn animal_make_love(partner_type: &'static str) -> Behavior {
    // `super(ImmutableMap.of(...), 110)` (`:27-41`).
    Behavior::with_duration(
        vec![
            (
                MemoryModuleType::NearestVisibleLivingEntities,
                MemoryStatus::ValuePresent,
            ),
            (MemoryModuleType::BreedTarget, MemoryStatus::ValueAbsent),
            (MemoryModuleType::WalkTarget, MemoryStatus::Registered),
            (MemoryModuleType::LookTarget, MemoryStatus::Registered),
            (MemoryModuleType::IsPanicking, MemoryStatus::ValueAbsent),
        ],
        DURATION,
        DURATION,
        AnimalMakeLove {
            partner_type,
            speed_modifier: 1.0,
            close_enough_distance: 2,
            spawn_child_at_time: 0,
        },
    )
}

#[cfg(test)]
mod tests;
