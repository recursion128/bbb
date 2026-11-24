//! The villager's household behaviours: finding another villager, breeding
//! with it, and the farmer's field.

use bbb_protocol::packets::BlockPos;

use super::super::effect::{BrainWorldEffect, VillagerHeldItem};
use super::super::memory::{MemoryModuleType, MemoryStatus, MemoryValue, MemoryWalkTarget};
use super::super::sensor::FarmCell;
use super::super::{Behavior, BehaviorHooks, BrainTick, BrainTickContext};
use super::animal_make_love::lock_gaze_and_walk_to_each_other;
use super::villager_work::{closer_to_center_than, global_pos, in_this_level};
use super::{body_sees_entity, entity_tracker, make_walk_target};

const VILLAGER: &str = "minecraft:villager";
/// `VillagerGoalPackages.INTERACT_SPEED_MODIFIER` / `INTERACT_WALKUP_DIST`
/// (`VillagerGoalPackages.java:29-30`).
const INTERACT_SPEED_MODIFIER: f32 = 0.5;
const INTERACT_WALKUP_DIST: i32 = 2;
const INTERACT_DIST_SQR: f64 = 5.0;

/// `BehaviorUtils.targetIsValid(brain, memory, VILLAGER)`
pub(super) fn villager_target(tick: &BrainTick<'_>, memory: MemoryModuleType) -> Option<i32> {
    let target = tick.memory_entity(memory)?;
    let facts = tick.world().entity_facts(target)?;
    (facts.entity_resource_id == VILLAGER && facts.alive && body_sees_entity(tick, target))
        .then_some(target)
}

fn queue_event(tick: &mut BrainTick<'_>, entity_id: i32, event_id: i8) {
    tick.world_mut()
        .queue_effect(BrainWorldEffect::EntityEvent {
            entity_id,
            event_id,
        });
}

/// Nearby villager/cat interaction, with breeding filters for `BreedTarget`.
#[derive(Debug, Clone, Copy)]
struct InteractWith {
    entity_type: &'static str,
    memory: MemoryModuleType,
}

impl BehaviorHooks for InteractWith {
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        let breeding = self.memory == MemoryModuleType::BreedTarget;
        let body = tick.world().body();
        if breeding && !body.villager.is_some_and(|villager| villager.can_breed) {
            return false;
        }
        let body_id = body.entity_id;
        let Some(MemoryValue::VisibleEntities(ids)) =
            tick.memory(MemoryModuleType::NearestVisibleLivingEntities)
        else {
            return false;
        };
        let closest = ids.iter().copied().find(|id| {
            *id != body_id
                && tick
                    .world()
                    .entity_facts(*id)
                    .is_some_and(|facts| facts.entity_resource_id == self.entity_type)
                && tick
                    .world()
                    .entity_distance_to_sqr(*id)
                    .is_some_and(|distance| distance <= 64.0)
                && (!breeding || tick.world().villager_can_breed(*id))
                && body_sees_entity(tick, *id)
        });
        let Some(target) = closest.and_then(|id| entity_tracker(tick, id).map(|t| (id, t))) else {
            return false;
        };
        let (target_id, tracker) = target;
        tick.set_memory_value(self.memory, MemoryValue::EntityRef(target_id));
        tick.set_memory_value(
            MemoryModuleType::LookTarget,
            MemoryValue::EntityTracker(target_id),
        );
        tick.set_memory_value(
            MemoryModuleType::WalkTarget,
            MemoryValue::WalkTarget(MemoryWalkTarget {
                target: tracker,
                speed_modifier: INTERACT_SPEED_MODIFIER,
                close_enough_dist: INTERACT_WALKUP_DIST,
            }),
        );
        true
    }

    fn debug_string(&self) -> &'static str {
        "InteractWith"
    }
}

/// `memory` is `INTERACTION_TARGET`, or `BREED_TARGET` for the breeding form.
pub fn interact_with_villager(memory: MemoryModuleType) -> Behavior {
    interact_with(memory, VILLAGER)
}

pub fn interact_with_cat() -> Behavior {
    interact_with(MemoryModuleType::InteractionTarget, "minecraft:cat")
}

fn interact_with(memory: MemoryModuleType, entity_type: &'static str) -> Behavior {
    Behavior::one_shot(
        vec![
            (memory, MemoryStatus::Registered),
            (MemoryModuleType::LookTarget, MemoryStatus::Registered),
            (MemoryModuleType::WalkTarget, MemoryStatus::ValueAbsent),
            (
                MemoryModuleType::NearestVisibleLivingEntities,
                MemoryStatus::ValuePresent,
            ),
        ],
        InteractWith {
            memory,
            entity_type,
        },
    )
}

/// `TradeWithVillager`; what is handed over
/// is decided where both inventories are live.
#[derive(Debug, Clone, Copy)]
pub struct TradeWithVillager;

impl BehaviorHooks for TradeWithVillager {
    fn extra_start_conditions(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
    ) -> bool {
        villager_target(tick, MemoryModuleType::InteractionTarget).is_some()
    }

    fn can_still_use(
        &self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        villager_target(tick, MemoryModuleType::InteractionTarget).is_some()
    }

    fn start(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) {
        if let Some(target) = tick.memory_entity(MemoryModuleType::InteractionTarget) {
            lock_gaze_and_walk_to_each_other(tick, target, 0.5, 2);
        }
    }

    fn tick(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, _timestamp: i64) {
        let Some(target) = tick.memory_entity(MemoryModuleType::InteractionTarget) else {
            return;
        };
        if tick
            .world()
            .entity_distance_to_sqr(target)
            .is_none_or(|distance| distance > INTERACT_DIST_SQR)
        {
            return;
        }
        lock_gaze_and_walk_to_each_other(tick, target, 0.5, 2);
        tick.world_mut()
            .queue_effect(BrainWorldEffect::TradeWithVillager { target_id: target });
    }

    fn stop(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, _timestamp: i64) {
        tick.erase_memory(MemoryModuleType::InteractionTarget);
    }

    fn debug_string(&self) -> &'static str {
        "TradeWithVillager"
    }
}

pub fn trade_with_villager() -> Behavior {
    Behavior::new(
        vec![
            (
                MemoryModuleType::InteractionTarget,
                MemoryStatus::ValuePresent,
            ),
            (
                MemoryModuleType::NearestVisibleLivingEntities,
                MemoryStatus::ValuePresent,
            ),
        ],
        TradeWithVillager,
    )
}

/// `VillagerMakeLove`.
#[derive(Debug, Clone, Copy, Default)]
pub struct VillagerMakeLove {
    birth_timestamp: i64,
}

impl VillagerMakeLove {
    /// `isBreedingPossible` (`:82-87`).
    fn partner(tick: &BrainTick<'_>) -> Option<i32> {
        let partner = villager_target(tick, MemoryModuleType::BreedTarget)?;
        let body = tick.world().body().villager?;
        (body.can_breed && tick.world().villager_can_breed(partner)).then_some(partner)
    }
}

impl BehaviorHooks for VillagerMakeLove {
    fn extra_start_conditions(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
    ) -> bool {
        Self::partner(tick).is_some()
    }

    fn can_still_use(
        &self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        timestamp: i64,
    ) -> bool {
        timestamp <= self.birth_timestamp && Self::partner(tick).is_some()
    }

    fn start(&mut self, tick: &mut BrainTick<'_>, context: &mut BrainTickContext, timestamp: i64) {
        let Some(partner) = tick.memory_entity(MemoryModuleType::BreedTarget) else {
            return;
        };
        lock_gaze_and_walk_to_each_other(tick, partner, 0.5, 2);
        let body = tick.world().body().entity_id;
        queue_event(tick, partner, 18);
        queue_event(tick, body, 18);
        self.birth_timestamp = timestamp + 275 + i64::from(context.next_int(50));
    }

    fn tick(&mut self, tick: &mut BrainTick<'_>, context: &mut BrainTickContext, timestamp: i64) {
        let Some(partner) = tick.memory_entity(MemoryModuleType::BreedTarget) else {
            return;
        };
        if tick
            .world()
            .entity_distance_to_sqr(partner)
            .is_none_or(|distance| distance > INTERACT_DIST_SQR)
        {
            return;
        }
        lock_gaze_and_walk_to_each_other(tick, partner, 0.5, 2);
        if timestamp >= self.birth_timestamp {
            tick.world_mut()
                .queue_effect(BrainWorldEffect::VillagerGiveBirth {
                    partner_id: partner,
                });
        } else if context.next_int(35) == 0 {
            let body = tick.world().body().entity_id;
            queue_event(tick, partner, 12);
            queue_event(tick, body, 12);
        }
    }

    fn stop(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, _timestamp: i64) {
        tick.erase_memory(MemoryModuleType::BreedTarget);
    }

    fn debug_string(&self) -> &'static str {
        "VillagerMakeLove"
    }
}

pub fn villager_make_love() -> Behavior {
    Behavior::with_duration(
        vec![
            (MemoryModuleType::BreedTarget, MemoryStatus::ValuePresent),
            (
                MemoryModuleType::NearestVisibleLivingEntities,
                MemoryStatus::ValuePresent,
            ),
        ],
        350,
        350,
        VillagerMakeLove::default(),
    )
}

/// `HarvestFarmland`.
#[derive(Debug, Clone, Default)]
pub struct HarvestFarmland {
    above_farmland: Option<BlockPos>,
    next_ok_start_time: i64,
    time_worked_so_far: i32,
    valid_farmland: Vec<BlockPos>,
}

/// `HarvestFarmland.HARVEST_DURATION` (`HarvestFarmland.java:28`).
const HARVEST_DURATION: i32 = 200;

impl HarvestFarmland {
    fn pick(&mut self, tick: &mut BrainTick<'_>) -> Option<BlockPos> {
        if self.valid_farmland.is_empty() {
            return None;
        }
        let bound = i32::try_from(self.valid_farmland.len()).unwrap_or(i32::MAX);
        let index = tick.world_mut().level_random_next_int(bound);
        self.valid_farmland
            .get(usize::try_from(index).ok()?)
            .copied()
    }

    fn walk_to(tick: &mut BrainTick<'_>, pos: BlockPos) {
        tick.set_memory_value(MemoryModuleType::LookTarget, MemoryValue::BlockPos(pos));
        tick.set_memory_value(
            MemoryModuleType::WalkTarget,
            MemoryValue::WalkTarget(MemoryWalkTarget::at(pos, 0.5, 1)),
        );
    }
}

impl BehaviorHooks for HarvestFarmland {
    fn extra_start_conditions(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
    ) -> bool {
        if !tick.world().mob_griefing()
            || tick
                .world()
                .body()
                .villager
                .is_none_or(|villager| villager.profession != "minecraft:farmer")
        {
            return false;
        }
        let center = super::containing_block_pos(tick.world().body().position);
        self.valid_farmland.clear();
        for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    let pos = BlockPos {
                        x: center.x + x,
                        y: center.y + y,
                        z: center.z + z,
                    };
                    if matches!(
                        tick.world().farm_cell(pos),
                        FarmCell::RipeCrop | FarmCell::EmptyFarmland
                    ) {
                        self.valid_farmland.push(pos);
                    }
                }
            }
        }
        self.above_farmland = self.pick(tick);
        self.above_farmland.is_some()
    }

    fn start(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, timestamp: i64) {
        if let Some(pos) = self
            .above_farmland
            .filter(|_| timestamp > self.next_ok_start_time)
        {
            Self::walk_to(tick, pos);
        }
    }

    fn stop(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, timestamp: i64) {
        tick.erase_memory(MemoryModuleType::LookTarget);
        tick.erase_memory(MemoryModuleType::WalkTarget);
        self.time_worked_so_far = 0;
        self.next_ok_start_time = timestamp + 40;
    }

    fn tick(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, timestamp: i64) {
        let position = tick.world().body().position;
        if self
            .above_farmland
            .is_some_and(|pos| !closer_to_center_than(pos, position, 1.0))
        {
            return;
        }
        if let Some(pos) = self
            .above_farmland
            .filter(|_| timestamp > self.next_ok_start_time)
        {
            let cell = tick.world().farm_cell(pos);
            let has_seeds = tick
                .world()
                .body()
                .villager
                .is_some_and(|villager| villager.has_farm_seeds);
            tick.world_mut()
                .queue_effect(BrainWorldEffect::VillagerFarmAt { pos });
            let will_plant = cell == FarmCell::EmptyFarmland && has_seeds;
            if cell != FarmCell::RipeCrop && !will_plant {
                self.valid_farmland.retain(|valid| *valid != pos);
                self.above_farmland = self.pick(tick);
                if let Some(next) = self.above_farmland {
                    self.next_ok_start_time = timestamp + 20;
                    Self::walk_to(tick, next);
                }
            }
        }
        self.time_worked_so_far += 1;
    }

    fn can_still_use(
        &self,
        _tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        self.time_worked_so_far < HARVEST_DURATION
    }

    fn debug_string(&self) -> &'static str {
        "HarvestFarmland"
    }
}

pub fn harvest_farmland() -> Behavior {
    Behavior::new(
        vec![
            (MemoryModuleType::LookTarget, MemoryStatus::ValueAbsent),
            (MemoryModuleType::WalkTarget, MemoryStatus::ValueAbsent),
            (
                MemoryModuleType::SecondaryJobSite,
                MemoryStatus::ValuePresent,
            ),
        ],
        HarvestFarmland::default(),
    )
}

/// `UseBonemeal`, with the body's game-time clock standing in for its
/// `tickCount`.
#[derive(Debug, Clone, Copy, Default)]
pub struct UseBonemeal {
    next_work_cycle_time: i64,
    last_bonemealing_session: i64,
    time_worked_so_far: i32,
    crop_pos: Option<BlockPos>,
}

/// `UseBonemeal.BONEMEALING_DURATION` (`UseBonemeal.java:22`).
const BONEMEALING_DURATION: i32 = 80;

impl UseBonemeal {
    /// `pickNextTarget`: a uniform pick among the
    /// growing crops around the body.
    fn pick_next_target(tick: &mut BrainTick<'_>) -> Option<BlockPos> {
        let center = tick.world().body().block_position;
        let mut result = None;
        let mut count = 0;
        for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    let pos = BlockPos {
                        x: center.x + x,
                        y: center.y + y,
                        z: center.z + z,
                    };
                    if tick.world().farm_cell(pos) == FarmCell::GrowingCrop {
                        count += 1;
                        if tick.world_mut().level_random_next_int(count) == 0 {
                            result = Some(pos);
                        }
                    }
                }
            }
        }
        result
    }

    fn target_crop(&self, tick: &mut BrainTick<'_>) {
        if let Some(pos) = self.crop_pos {
            HarvestFarmland::walk_to(tick, pos);
        }
    }

    fn has_bone_meal(tick: &BrainTick<'_>) -> bool {
        tick.world()
            .body()
            .villager
            .is_some_and(|villager| villager.has_bone_meal)
    }
}

impl BehaviorHooks for UseBonemeal {
    fn extra_start_conditions(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
    ) -> bool {
        let now = tick.world().game_time();
        if now % 10 != 0
            || (self.last_bonemealing_session != 0 && self.last_bonemealing_session + 160 > now)
            || !Self::has_bone_meal(tick)
        {
            return false;
        }
        self.crop_pos = Self::pick_next_target(tick);
        self.crop_pos.is_some()
    }

    fn can_still_use(
        &self,
        _tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        self.time_worked_so_far < BONEMEALING_DURATION && self.crop_pos.is_some()
    }

    fn start(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, timestamp: i64) {
        self.target_crop(tick);
        tick.world_mut()
            .queue_effect(BrainWorldEffect::VillagerHeldItem(
                VillagerHeldItem::BoneMeal,
            ));
        self.next_work_cycle_time = timestamp;
        self.time_worked_so_far = 0;
    }

    fn stop(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, _timestamp: i64) {
        tick.world_mut()
            .queue_effect(BrainWorldEffect::VillagerHeldItem(VillagerHeldItem::Clear));
        self.last_bonemealing_session = tick.world().game_time();
    }

    fn tick(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, timestamp: i64) {
        let Some(pos) = self.crop_pos else {
            return;
        };
        if timestamp < self.next_work_cycle_time
            || !closer_to_center_than(pos, tick.world().body().position, 1.0)
        {
            return;
        }
        if Self::has_bone_meal(tick) && tick.world().farm_cell(pos) == FarmCell::GrowingCrop {
            tick.world_mut()
                .queue_effect(BrainWorldEffect::VillagerBonemealAt { pos });
            self.crop_pos = Self::pick_next_target(tick);
            self.target_crop(tick);
            self.next_work_cycle_time = timestamp + 40;
        }
        self.time_worked_so_far += 1;
    }

    fn debug_string(&self) -> &'static str {
        "UseBonemeal"
    }
}

pub fn use_bonemeal() -> Behavior {
    Behavior::new(
        vec![
            (MemoryModuleType::LookTarget, MemoryStatus::ValueAbsent),
            (MemoryModuleType::WalkTarget, MemoryStatus::ValueAbsent),
        ],
        UseBonemeal::default(),
    )
}

/// `StrollAroundPoi`: near the remembered POI,
/// a short random walk every 180 ticks.
#[derive(Debug, Clone, Copy)]
pub struct StrollAroundPoi {
    memory: MemoryModuleType,
    speed_modifier: f32,
    max_distance: f64,
    next_ok_start_time: i64,
}

impl BehaviorHooks for StrollAroundPoi {
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        context: &mut BrainTickContext,
        timestamp: i64,
    ) -> bool {
        let Some(poi) = global_pos(tick, self.memory) else {
            return false;
        };
        if !in_this_level(tick, &poi)
            || !closer_to_center_than(poi.pos, tick.world().body().position, self.max_distance)
        {
            return false;
        }
        if timestamp <= self.next_ok_start_time {
            return true;
        }
        match tick
            .world_mut()
            .body_land_random_pos(context.body_random(), 8, 6)
            .flatten()
        {
            Some(position) => tick.set_memory_value(
                MemoryModuleType::WalkTarget,
                MemoryValue::WalkTarget(make_walk_target(position, self.speed_modifier, 1)),
            ),
            None => tick.erase_memory(MemoryModuleType::WalkTarget),
        }
        self.next_ok_start_time = timestamp + 180;
        true
    }

    fn debug_string(&self) -> &'static str {
        "StrollAroundPoi"
    }
}

pub fn stroll_around_poi(
    memory: MemoryModuleType,
    speed_modifier: f32,
    max_distance: i32,
) -> Behavior {
    Behavior::one_shot(
        vec![
            (MemoryModuleType::WalkTarget, MemoryStatus::Registered),
            (memory, MemoryStatus::ValuePresent),
        ],
        StrollAroundPoi {
            memory,
            speed_modifier,
            max_distance: f64::from(max_distance),
            next_ok_start_time: 0,
        },
    )
}

/// `SocializeAtBell`: now and then a villager
/// at the meeting point walks up to the closest villager there.
#[derive(Debug, Clone, Copy)]
pub struct SocializeAtBell;

impl BehaviorHooks for SocializeAtBell {
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        let Some(meeting) = global_pos(tick, MemoryModuleType::MeetingPoint) else {
            return false;
        };
        if tick.world_mut().level_random_next_int(100) != 0
            || !in_this_level(tick, &meeting)
            || !closer_to_center_than(meeting.pos, tick.world().body().position, 4.0)
        {
            return false;
        }
        let body_id = tick.world().body().entity_id;
        let Some(MemoryValue::VisibleEntities(ids)) =
            tick.memory(MemoryModuleType::NearestVisibleLivingEntities)
        else {
            return false;
        };
        let mut saw_visible_villager = false;
        let mut closest = None;
        for &id in ids {
            if id == body_id
                || !tick
                    .world()
                    .entity_facts(id)
                    .is_some_and(|facts| facts.entity_resource_id == VILLAGER)
                || !body_sees_entity(tick, id)
            {
                continue;
            }
            saw_visible_villager = true;
            if tick
                .world()
                .entity_distance_to_sqr(id)
                .is_some_and(|distance| distance <= 32.0)
            {
                closest = Some(id);
                break;
            }
        }
        if !saw_visible_villager {
            return false;
        }
        if let Some((target_id, tracker)) =
            closest.and_then(|id| entity_tracker(tick, id).map(|tracker| (id, tracker)))
        {
            tick.set_memory_value(
                MemoryModuleType::InteractionTarget,
                MemoryValue::EntityRef(target_id),
            );
            tick.set_memory_value(
                MemoryModuleType::LookTarget,
                MemoryValue::EntityTracker(target_id),
            );
            tick.set_memory_value(
                MemoryModuleType::WalkTarget,
                MemoryValue::WalkTarget(MemoryWalkTarget {
                    target: tracker,
                    speed_modifier: 0.3,
                    close_enough_dist: 1,
                }),
            );
        }
        true
    }

    fn debug_string(&self) -> &'static str {
        "SocializeAtBell"
    }
}

pub fn socialize_at_bell() -> Behavior {
    Behavior::one_shot(
        vec![
            (MemoryModuleType::WalkTarget, MemoryStatus::Registered),
            (MemoryModuleType::LookTarget, MemoryStatus::Registered),
            (MemoryModuleType::MeetingPoint, MemoryStatus::ValuePresent),
            (
                MemoryModuleType::NearestVisibleLivingEntities,
                MemoryStatus::ValuePresent,
            ),
            (
                MemoryModuleType::InteractionTarget,
                MemoryStatus::ValueAbsent,
            ),
        ],
        SocializeAtBell,
    )
}
