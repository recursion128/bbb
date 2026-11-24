//! The young villager's daytime play: friends, wandering, and jumping on beds.

use rustc_hash::FxHashMap;

use super::super::{Behavior, BehaviorHooks, BrainTick, BrainTickContext};
use super::{body_sees_entity, containing_block_pos, entity_tracker, make_walk_target};
use super::{MemoryModuleType, MemoryStatus, MemoryValue, MemoryWalkTarget};

#[derive(Debug, Clone, Copy)]
struct PlayTagWithOtherKids;

impl BehaviorHooks for PlayTagWithOtherKids {
    // `PlayTagWithOtherKids.create`.
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        if !tick.world().body().villager.is_some_and(|facts| facts.baby)
            || context.body_random().next_int(10) != 0
        {
            return false;
        }
        let body_id = tick.world().body().entity_id;
        let mut first_friend = None;
        let mut being_chased = false;
        let mut chasers = FxHashMap::<i32, usize>::default();
        for (id, target) in nearby_ids(tick)
            .iter()
            .filter_map(|id| visible_child(tick, *id))
        {
            first_friend.get_or_insert(id);
            if target == Some(body_id) {
                being_chased = true;
                break;
            }
            if let Some(target) = target {
                *chasers.entry(target).or_default() += 1;
            }
        }
        let Some(first_friend) = first_friend else {
            return false;
        };
        if being_chased {
            for _ in 0..10 {
                if let Some(Some(pos)) =
                    tick.world_mut()
                        .body_land_random_pos(context.body_random(), 20, 8)
                {
                    if tick.world().is_village(containing_block_pos(pos)) {
                        tick.set_memory_value(
                            MemoryModuleType::WalkTarget,
                            MemoryValue::WalkTarget(make_walk_target(pos, 0.6, 0)),
                        );
                        break;
                    }
                }
            }
            return true;
        }
        // Join the least crowded chase, including a friend temporarily out of sight.
        let target = chasers
            .into_iter()
            .filter(|(id, count)| {
                *count <= 5
                    && tick
                        .world()
                        .villager_play_facts(*id)
                        .is_some_and(|facts| facts.baby)
            })
            .min_by_key(|(_, count)| *count)
            .map_or(first_friend, |(id, _)| id);
        if let Some(tracker) = entity_tracker(tick, target) {
            tick.set_memory_value(
                MemoryModuleType::InteractionTarget,
                MemoryValue::EntityRef(target),
            );
            tick.set_memory_value(
                MemoryModuleType::LookTarget,
                MemoryValue::EntityTracker(target),
            );
            tick.set_memory_value(
                MemoryModuleType::WalkTarget,
                MemoryValue::WalkTarget(MemoryWalkTarget {
                    target: tracker,
                    speed_modifier: 0.6,
                    close_enough_dist: 1,
                }),
            );
        }
        true
    }

    fn debug_string(&self) -> &'static str {
        "PlayTagWithOtherKids"
    }
}

pub fn play_tag_with_other_kids() -> Behavior {
    Behavior::one_shot(
        vec![
            (MemoryModuleType::WalkTarget, MemoryStatus::ValueAbsent),
            (
                MemoryModuleType::NearestVisibleLivingEntities,
                MemoryStatus::Registered,
            ),
            (MemoryModuleType::LookTarget, MemoryStatus::Registered),
            (
                MemoryModuleType::InteractionTarget,
                MemoryStatus::Registered,
            ),
        ],
        PlayTagWithOtherKids,
    )
}

fn nearby_ids<'a>(tick: &'a BrainTick<'_>) -> &'a [i32] {
    match tick.memory(MemoryModuleType::NearestVisibleLivingEntities) {
        Some(MemoryValue::VisibleEntities(ids)) => ids.as_slice(),
        _ => &[],
    }
}

fn visible_child(tick: &BrainTick<'_>, id: i32) -> Option<(i32, Option<i32>)> {
    let facts = tick.world().villager_play_facts(id)?;
    (facts.baby && body_sees_entity(tick, id)).then_some((id, facts.interaction_target))
}

#[derive(Clone)]
struct PlayFallback {
    next_attempt: i64,
    selection: Behavior,
}

impl BehaviorHooks for PlayFallback {
    fn for_each_required_memory(&self, visit: &mut dyn FnMut(MemoryModuleType)) {
        self.selection.for_each_required_memory(visit);
    }
    fn start(&mut self, tick: &mut BrainTick<'_>, context: &mut BrainTickContext, timestamp: i64) {
        if timestamp < self.next_attempt {
            return;
        }
        self.next_attempt = timestamp + 20;
        if tick.world().body().villager.is_some_and(|facts| facts.baby)
            && !nearby_ids(tick)
                .iter()
                .any(|id| visible_child(tick, *id).is_some())
        {
            self.selection.try_start(tick, context, timestamp);
        }
    }
    fn tick(&mut self, tick: &mut BrainTick<'_>, context: &mut BrainTickContext, timestamp: i64) {
        if !tick.is_active(super::super::Activity::PLAY)
            || !tick.world().body().villager.is_some_and(|facts| facts.baby)
        {
            self.stop(tick, context, timestamp);
        } else if self.selection.status() == super::super::BehaviorStatus::Running {
            self.selection.tick_or_stop(tick, context, timestamp);
        }
    }
    fn stop(&mut self, tick: &mut BrainTick<'_>, context: &mut BrainTickContext, timestamp: i64) {
        if self.selection.status() == super::super::BehaviorStatus::Running {
            self.selection.do_stop(tick, context, timestamp);
        }
    }
    fn stops_after_tick(&self) -> bool {
        self.selection.status() == super::super::BehaviorStatus::Stopped
    }
    fn debug_string(&self) -> &'static str {
        "VillagerPlayFallback"
    }
}

// `VillagerGoalPackages.getPlayPackage`.
pub fn villager_play_fallback(speed: f32) -> Behavior {
    Behavior::gate(
        Vec::new(),
        PlayFallback {
            next_attempt: 0,
            selection: super::run_one(
                Vec::new(),
                [
                    (
                        2,
                        super::villager_life::interact_with_villager(
                            MemoryModuleType::InteractionTarget,
                        ),
                    ),
                    (1, super::villager_life::interact_with_cat()),
                    (
                        1,
                        super::villager_stroll::village_bound_random_stroll(speed),
                    ),
                    (1, super::set_walk_target_from_look_target(speed, 2)),
                    (2, jump_on_bed(speed)),
                    (2, super::do_nothing(20, 40)),
                ],
            ),
        },
    )
}

#[derive(Clone)]
struct JumpOnBed {
    speed: f32,
    next_search: i64,
    bed: Option<bbb_protocol::packets::BlockPos>,
    walking_left: i32,
    jumps_left: i32,
    cooldown: i32,
}

impl JumpOnBed {
    fn on_bed(&self, tick: &BrainTick<'_>) -> bool {
        tick.world()
            .bed_occupied(tick.world().body().block_position)
            .is_some()
    }
    fn over_bed(&self, tick: &BrainTick<'_>) -> bool {
        let pos = tick.world().body().block_position;
        self.on_bed(tick)
            || tick
                .world()
                .bed_occupied(bbb_protocol::packets::BlockPos {
                    y: pos.y - 1,
                    ..pos
                })
                .is_some()
    }
}

impl BehaviorHooks for JumpOnBed {
    fn extra_start_conditions(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
    ) -> bool {
        let now = tick.world().game_time();
        if now < self.next_search || !tick.world().body().villager.is_some_and(|facts| facts.baby) {
            return false;
        }
        self.next_search = now + 20;
        let center = tick.world().body().block_position;
        // `NearestBedSensor.doTick`, capped at 4 path probes per scan.
        self.bed = tick
            .world_mut()
            .home_pois_within(center, 48)
            .into_iter()
            .filter(|pos| tick.world().bed_occupied(*pos).is_some())
            .take(4)
            .find(|pos| tick.world().body_path_reaches(*pos, 1, 48.0) == Some(true));
        self.bed.is_some()
    }
    // `JumpOnBed.start`.
    fn start(&mut self, tick: &mut BrainTick<'_>, context: &mut BrainTickContext, _timestamp: i64) {
        self.walking_left = 100;
        self.jumps_left = 3 + context.body_random().next_int(4);
        self.cooldown = 0;
        if let Some(bed) = self.bed {
            tick.set_memory_value(
                MemoryModuleType::WalkTarget,
                MemoryValue::WalkTarget(MemoryWalkTarget::at(bed, self.speed, 0)),
            );
        }
    }
    fn can_still_use(
        &self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        tick.world().body().villager.is_some_and(|facts| facts.baby)
            && self
                .bed
                .is_some_and(|bed| tick.world().bed_occupied(bed).is_some())
            && if self.over_bed(tick) {
                self.jumps_left > 0
            } else {
                self.walking_left > 0
            }
    }
    // `JumpOnBed.tick`.
    fn tick(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, _timestamp: i64) {
        if !self.over_bed(tick) {
            self.walking_left -= 1;
        } else if self.cooldown > 0 {
            self.cooldown -= 1;
        } else if self.on_bed(tick) {
            tick.world_mut().request_body_jump();
            self.jumps_left -= 1;
            self.cooldown = 5;
        }
    }
    fn stop(
        &mut self,
        _tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) {
        self.bed = None;
    }
    fn timed_out(&self, _timestamp: i64, _end_timestamp: i64) -> bool {
        false
    }
    fn debug_string(&self) -> &'static str {
        "JumpOnBed"
    }
}

pub fn jump_on_bed(speed: f32) -> Behavior {
    Behavior::new(
        vec![(MemoryModuleType::WalkTarget, MemoryStatus::ValueAbsent)],
        JumpOnBed {
            speed,
            next_search: 0,
            bed: None,
            walking_left: 0,
            jumps_left: 0,
            cooldown: 0,
        },
    )
}

// `VillagerGoalPackages.getMinimalLookBehavior`.
pub fn villager_minimal_look() -> Behavior {
    use super::LookTargetFilter::Resource;
    super::run_one(
        Vec::new(),
        [
            (
                2,
                super::set_entity_look_target(Resource("minecraft:villager"), 8.0),
            ),
            (
                2,
                super::set_entity_look_target(Resource("minecraft:player"), 8.0),
            ),
            (8, super::do_nothing(30, 60)),
        ],
    )
}

// `VillagerGoalPackages.getFullLookBehavior`.
pub fn villager_full_look() -> Behavior {
    use super::LookTargetFilter::{Category as MobCategory, Resource};
    use bbb_protocol::entity_types::VanillaEntityMobCategory as Category;
    super::run_one(
        Vec::new(),
        [
            (
                8,
                super::set_entity_look_target(Resource("minecraft:cat"), 8.0),
            ),
            (
                2,
                super::set_entity_look_target(Resource("minecraft:villager"), 8.0),
            ),
            (
                2,
                super::set_entity_look_target(Resource("minecraft:player"), 8.0),
            ),
            (
                1,
                super::set_entity_look_target(MobCategory(Category::Creature), 8.0),
            ),
            (
                1,
                super::set_entity_look_target(MobCategory(Category::WaterCreature), 8.0),
            ),
            (
                1,
                super::set_entity_look_target(MobCategory(Category::Axolotls), 8.0),
            ),
            (
                1,
                super::set_entity_look_target(MobCategory(Category::UndergroundWaterCreature), 8.0),
            ),
            (
                1,
                super::set_entity_look_target(MobCategory(Category::WaterAmbient), 8.0),
            ),
            (
                1,
                super::set_entity_look_target(MobCategory(Category::Monster), 8.0),
            ),
            (2, super::do_nothing(30, 60)),
        ],
    )
}
