mod behaviors;
mod effect;
mod memory;
mod sensor;
mod sensors;

use rustc_hash::FxHashMap;
use std::collections::BTreeMap;
use std::fmt;

use crate::random::GameRandom;
use bbb_facts::activity_types::{vanilla_activity_for_resource_id, VANILLA_ACTIVITIES};

pub use behaviors::{
    acquire_poi, assign_profession_from_job_site, go_to_potential_job_site,
    look_and_follow_trading_player, poi_competitor_scan, reset_profession,
    set_walk_target_from_block_memory, sleep_in_bed, stroll_to_poi, stroll_to_poi_list,
    validate_nearby_poi, wake_up, work_at_poi, yield_job_site,
};
pub use behaviors::{
    animal_make_love, animal_panic, count_down_cooldown_ticks, croak, do_nothing,
    follow_temptation, long_jump_mid_jump, long_jump_to_preferred_block, look_at_target_sink,
    melee_attack, move_to_target_sink, ordered_try_all, random_stroll, random_stroll_swim, run_one,
    set_entity_look_target_sometimes, set_entity_look_target_sometimes_of,
    set_walk_target_from_attack_target_if_target_out_of_reach, set_walk_target_from_look_target,
    shoot_tongue, start_attacking, start_attacking_when_active, stop_attacking_if_target_invalid,
    stop_attacking_if_target_invalid_unless_in, swim, trigger_if_in_water, trigger_if_on_ground,
    try_find_land, try_find_land_near_water, try_lay_spawn_on_fluid_near_land, AnimalMakeLove,
    AnimalPanic, CountDownCooldownTicks, Croak, DoNothing, FollowTemptation, LongJumpMidJump,
    LongJumpToPreferredBlock, LookAtTargetSink, MeleeAttack, MoveToTargetSink, RandomStroll,
    RunOne, SetEntityLookTargetSometimes, SetWalkTargetFromAttackTargetIfTargetOutOfReach,
    SetWalkTargetFromLookTarget, ShootTongue, StartAttacking, StartAttackingPredicate,
    StopAttackingIfTargetInvalid, Swim, TriggerIfOnGround, TryFindLand, TryFindLandNearWater,
    TryLaySpawnOnFluidNearLand, MOB_ATTACK_DAMAGE_TYPE,
};
pub use behaviors::{
    go_to_closest_village, inside_brownian_walk, set_closest_home_as_walk_target,
    set_look_and_interact_player, show_trades_to_player, use_bonemeal,
};
pub use behaviors::{
    harvest_farmland, interact_with_villager, locate_hiding_place, move_to_sky_seeing_spot,
    play_tag_with_other_kids, set_hidden_state, set_walk_target_away_from, socialize_at_bell,
    stroll_around_poi, trade_with_villager, village_bound_random_stroll,
    village_bound_random_stroll_within, villager_calm_down, villager_full_look, villager_make_love,
    villager_minimal_look, villager_panic_trigger, villager_play_fallback,
};
pub use behaviors::{interact_with_cat, jump_on_bed};
pub use effect::{BrainWorldEffect, VillagerHeldItem};
pub use memory::{
    BrainCodecError, BrainDamageSource, BrainPacked, MemoryGlobalPos, MemoryModuleType, MemoryPath,
    MemoryPositionTracker, MemoryStatus, MemoryValue, MemoryValueKind, MemoryWalkTarget,
    PackedMemory,
};
pub use sensor::{
    BrainBlockFacts, BrainBreedFacts, BrainEntityFacts, BrainPlayer, BrainWorld,
    EntityTargetingTest, FarmCell, SensorType, VillagerJobFacts, VillagerPlayFacts,
    DEFAULT_SCAN_RATE,
};

use memory::{apply_value_to_slot, MemorySlot};
use sensor::SensorEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Activity {
    id: i32,
}

impl Activity {
    pub const CORE: Self = Self { id: 0 };
    pub const IDLE: Self = Self { id: 1 };
    pub const WORK: Self = Self { id: 2 };
    pub const PLAY: Self = Self { id: 3 };
    pub const REST: Self = Self { id: 4 };
    pub const MEET: Self = Self { id: 5 };
    pub const PANIC: Self = Self { id: 6 };
    pub const RAID: Self = Self { id: 7 };
    pub const PRE_RAID: Self = Self { id: 8 };
    pub const HIDE: Self = Self { id: 9 };
    pub const FIGHT: Self = Self { id: 10 };
    pub const CELEBRATE: Self = Self { id: 11 };
    pub const ADMIRE_ITEM: Self = Self { id: 12 };
    pub const AVOID: Self = Self { id: 13 };
    pub const RIDE: Self = Self { id: 14 };
    pub const PLAY_DEAD: Self = Self { id: 15 };
    pub const LONG_JUMP: Self = Self { id: 16 };
    pub const RAM: Self = Self { id: 17 };
    pub const TONGUE: Self = Self { id: 18 };
    pub const SWIM: Self = Self { id: 19 };
    pub const LAY_SPAWN: Self = Self { id: 20 };
    pub const SNIFF: Self = Self { id: 21 };
    pub const INVESTIGATE: Self = Self { id: 22 };
    pub const ROAR: Self = Self { id: 23 };
    pub const EMERGE: Self = Self { id: 24 };
    pub const DIG: Self = Self { id: 25 };

    pub fn from_id(id: i32) -> Option<Self> {
        (id >= 0 && (id as usize) < VANILLA_ACTIVITIES.len()).then_some(Self { id })
    }

    pub fn from_resource_id(resource_id: &str) -> Option<Self> {
        vanilla_activity_for_resource_id(resource_id).map(|entry| Self { id: entry.id })
    }

    pub const fn id(self) -> i32 {
        self.id
    }

    pub fn resource_id(self) -> &'static str {
        VANILLA_ACTIVITIES[self.id as usize].resource_id
    }
}

impl fmt::Display for Activity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.resource_id())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BehaviorStatus {
    Stopped,
    Running,
}

/// The game time and the gameplay random for one brain tick.
#[derive(Debug, Clone)]
pub struct BrainTickContext {
    pub(crate) game_time: i64,
    pub(crate) random: GameRandom,
}

impl BrainTickContext {
    pub(crate) fn new(game_time: i64, random: GameRandom) -> Self {
        Self { game_time, random }
    }

    pub fn game_time(&self) -> i64 {
        self.game_time
    }

    pub fn next_int(&mut self, bound: i32) -> i32 {
        self.random.next_int(bound)
    }

    pub fn next_float(&mut self) -> f32 {
        self.random.next_float()
    }

    pub fn next_double(&mut self) -> f64 {
        self.random.next_double()
    }

    pub fn body_random(&mut self) -> &GameRandom {
        &mut self.random
    }
}

#[cfg(test)]
impl Default for BrainTickContext {
    fn default() -> Self {
        Self::new(0, GameRandom::with_seed(0))
    }
}

/// The brain's memory slots and the world for one tick: what Vanilla's
/// `Sensor.doTick(ServerLevel level, E body)` and
/// `Behavior` bodies read and write.
pub struct BrainTick<'a> {
    memories: &'a mut FxHashMap<MemoryModuleType, MemorySlot>,
    activity: Activity,
    world: &'a mut dyn BrainWorld,
}

impl<'a> BrainTick<'a> {
    fn new(
        memories: &'a mut FxHashMap<MemoryModuleType, MemorySlot>,
        activity: Activity,
        world: &'a mut dyn BrainWorld,
    ) -> Self {
        Self {
            memories,
            activity,
            world,
        }
    }

    /// `Brain.isActive`.
    pub fn is_active(&self, activity: Activity) -> bool {
        activity == Activity::CORE || activity == self.activity
    }

    /// The world and body half of the tick's arguments, to read.
    pub fn world(&self) -> &dyn BrainWorld {
        self.world
    }

    /// The same half, to act on: Vanilla's bodies reach their own mob, which a
    /// stored brain cannot borrow between ticks.
    pub fn world_mut(&mut self) -> &mut dyn BrainWorld {
        self.world
    }

    /// `Brain.getMemory`: a memory the brain never
    /// registered panics rather than answering empty.
    pub fn memory(&self, memory_type: MemoryModuleType) -> Option<&MemoryValue> {
        memory_slot(self.memories, memory_type).value()
    }

    pub(super) fn memory_entity(&self, memory_type: MemoryModuleType) -> Option<i32> {
        match self.memory(memory_type) {
            Some(MemoryValue::EntityRef(entity_id)) => Some(*entity_id),
            _ => None,
        }
    }

    pub fn has_memory_value(&self, memory_type: MemoryModuleType) -> bool {
        self.check_memory(memory_type, MemoryStatus::ValuePresent)
    }

    /// `Brain.isMemoryValue`.
    pub fn is_memory_value(&self, memory_type: MemoryModuleType, value: &MemoryValue) -> bool {
        self.memories
            .get(&memory_type)
            .is_some_and(|slot| slot.value() == Some(value))
    }

    /// `Brain.checkMemory`.
    pub fn check_memory(&self, memory_type: MemoryModuleType, status: MemoryStatus) -> bool {
        check_memory(self.memories, memory_type, status)
    }

    pub fn time_until_expiry(&self, memory_type: MemoryModuleType) -> i64 {
        memory_slot(self.memories, memory_type).time_to_live()
    }

    /// `Brain.setMemory`.
    pub fn set_memory(&mut self, memory_type: MemoryModuleType, value: Option<MemoryValue>) {
        set_memory_internal(self.memories, memory_type, value, None);
    }

    pub fn set_memory_value(&mut self, memory_type: MemoryModuleType, value: MemoryValue) {
        set_memory_internal(self.memories, memory_type, Some(value), None);
    }

    /// `Brain.setMemoryWithExpiry`.
    pub fn set_memory_with_expiry(
        &mut self,
        memory_type: MemoryModuleType,
        value: MemoryValue,
        time_to_live: i64,
    ) {
        set_memory_internal(self.memories, memory_type, Some(value), Some(time_to_live));
    }

    /// `Brain.eraseMemory`.
    pub fn erase_memory(&mut self, memory_type: MemoryModuleType) {
        if let Some(slot) = self.memories.get_mut(&memory_type) {
            slot.clear();
        }
    }
}

#[inline]
fn memory_slot(
    memories: &FxHashMap<MemoryModuleType, MemorySlot>,
    memory_type: MemoryModuleType,
) -> &MemorySlot {
    memories
        .get(&memory_type)
        .unwrap_or_else(|| panic!("Unregistered memory fetched: {memory_type}"))
}

#[inline(always)]
fn check_memory(
    memories: &FxHashMap<MemoryModuleType, MemorySlot>,
    memory_type: MemoryModuleType,
    status: MemoryStatus,
) -> bool {
    let Some(slot) = memories.get(&memory_type) else {
        return false;
    };
    match status {
        MemoryStatus::Registered => true,
        MemoryStatus::ValuePresent => slot.has_value(),
        MemoryStatus::ValueAbsent => !slot.has_value(),
    }
}

/// `Brain.setMemoryInternal`: a memory the brain never
/// registered is ignored, and an empty collection erases the slot.
fn set_memory_internal(
    memories: &mut FxHashMap<MemoryModuleType, MemorySlot>,
    memory_type: MemoryModuleType,
    value: Option<MemoryValue>,
    time_to_live: Option<i64>,
) {
    let Some(slot) = memories.get_mut(&memory_type) else {
        return;
    };
    match value {
        Some(value) => apply_value_to_slot(slot, value, time_to_live),
        None => slot.clear(),
    }
}

pub trait BehaviorHooksClone {
    fn clone_box(&self) -> Box<dyn BehaviorHooks>;
}

impl<T: BehaviorHooks + Clone + 'static> BehaviorHooksClone for T {
    fn clone_box(&self) -> Box<dyn BehaviorHooks> {
        Box::new(self.clone())
    }
}

pub trait BehaviorHooks: Send + BehaviorHooksClone {
    fn for_each_required_memory(&self, _visit: &mut dyn FnMut(MemoryModuleType)) {}

    /// `OneShot.trigger`, including its memory writes.
    fn trigger(
        &mut self,
        _tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        false
    }

    /// `Behavior.checkExtraStartConditions`, which
    /// `Behavior.tryStart` asks after the entry condition
    /// and before the duration draw.
    ///
    /// Vanilla hands this the level and the body, and a `GateBehavior` draws
    /// from them here, so the context is mutable.
    fn extra_start_conditions(
        &mut self,
        _tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
    ) -> bool {
        true
    }

    fn start(
        &mut self,
        _tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) {
    }

    fn tick(
        &mut self,
        _tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) {
    }

    /// The duration `Behavior.tryStart` stamps into `endTimestamp`
    /// `minDuration + nextInt(max - min + 1)`.
    fn start_duration(
        &self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        min_duration: i32,
        max_duration: i32,
    ) -> i32 {
        min_duration
            + tick
                .world_mut()
                .level_random_next_int(max_duration + 1 - min_duration)
    }

    /// `Behavior.stop`, which Vanilla hands the level:
    /// `MoveToTargetSink` draws its retry delay from the level's random there
    /// so this hook carries the tick's random.
    fn stop(
        &mut self,
        _tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) {
    }

    /// `Behavior.canStillUse`, which Vanilla hands
    /// the level and the body: `LongJumpToRandomPos.canStillUse` writes the
    /// aborted jump's cooldown through them
    fn can_still_use(
        &self,
        _tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        false
    }

    /// `Behavior.timedOut`, which a behaviour may
    /// refuse, as `CountDownCooldownTicks` does
    fn timed_out(&self, timestamp: i64, end_timestamp: i64) -> bool {
        timestamp > end_timestamp
    }

    /// `GateBehavior.tickOrStop`, which ends a gate
    /// inside the tick that ends its last running child: a gate owns no
    /// duration of its own, so this is the only arm that stops one. A
    /// behaviour without children answers false, the default.
    fn stops_after_tick(&self) -> bool {
        false
    }

    fn debug_string(&self) -> &'static str {
        "Behavior"
    }
}

impl BehaviorHooks for () {}

impl Clone for Box<dyn BehaviorHooks> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

pub struct Behavior {
    entry_condition: Vec<(MemoryModuleType, MemoryStatus)>,
    kind: BehaviorKind,
    status: BehaviorStatus,
    end_timestamp: i64,
    min_duration: i32,
    max_duration: i32,
    hooks: Box<dyn BehaviorHooks>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BehaviorKind {
    Timed,
    OneShot,
    Gate,
}

impl Behavior {
    pub const DEFAULT_DURATION: i32 = 60;

    pub fn new(
        entry_condition: Vec<(MemoryModuleType, MemoryStatus)>,
        hooks: impl BehaviorHooks + 'static,
    ) -> Self {
        Self::with_duration(
            entry_condition,
            Self::DEFAULT_DURATION,
            Self::DEFAULT_DURATION,
            hooks,
        )
    }

    pub fn with_duration(
        entry_condition: Vec<(MemoryModuleType, MemoryStatus)>,
        min_duration: i32,
        max_duration: i32,
        hooks: impl BehaviorHooks + 'static,
    ) -> Self {
        Self::with_kind(
            entry_condition,
            min_duration,
            max_duration,
            BehaviorKind::Timed,
            hooks,
        )
    }

    pub fn one_shot(
        entry_condition: Vec<(MemoryModuleType, MemoryStatus)>,
        hooks: impl BehaviorHooks + 'static,
    ) -> Self {
        Self::with_kind(entry_condition, 0, 0, BehaviorKind::OneShot, hooks)
    }

    pub fn gate(
        entry_condition: Vec<(MemoryModuleType, MemoryStatus)>,
        hooks: impl BehaviorHooks + 'static,
    ) -> Self {
        Self::with_kind(entry_condition, 0, 0, BehaviorKind::Gate, hooks)
    }

    fn with_kind(
        entry_condition: Vec<(MemoryModuleType, MemoryStatus)>,
        min_duration: i32,
        max_duration: i32,
        kind: BehaviorKind,
        hooks: impl BehaviorHooks + 'static,
    ) -> Self {
        assert!(
            min_duration <= max_duration,
            "behavior duration range is inverted"
        );
        Self {
            entry_condition,
            kind,
            status: BehaviorStatus::Stopped,
            end_timestamp: 0,
            min_duration,
            max_duration,
            hooks: Box::new(hooks),
        }
    }

    pub fn status(&self) -> BehaviorStatus {
        self.status
    }

    pub fn debug_string(&self) -> &'static str {
        self.hooks.debug_string()
    }

    fn for_each_required_memory(&self, visit: &mut dyn FnMut(MemoryModuleType)) {
        for (memory_type, _) in &self.entry_condition {
            visit(*memory_type);
        }
        self.hooks.for_each_required_memory(visit);
    }

    /// `Behavior.hasRequiredMemories`, which
    /// `Behavior.tryStart` asks first.
    fn entry_conditions_met(&self, tick: &BrainTick<'_>) -> bool {
        self.entry_condition
            .iter()
            .all(|(memory_type, status)| tick.check_memory(*memory_type, *status))
    }

    fn try_start(
        &mut self,
        tick: &mut BrainTick<'_>,
        context: &mut BrainTickContext,
        timestamp: i64,
    ) -> bool {
        if !self.entry_conditions_met(tick) {
            return false;
        }
        match self.kind {
            BehaviorKind::Timed => {
                if !self.hooks.extra_start_conditions(tick, context) {
                    return false;
                }
                self.status = BehaviorStatus::Running;
                let duration =
                    self.hooks
                        .start_duration(tick, context, self.min_duration, self.max_duration);
                self.end_timestamp = timestamp + i64::from(duration);
                self.hooks.start(tick, context, timestamp);
                true
            }
            BehaviorKind::OneShot => {
                if !self.hooks.trigger(tick, context, timestamp) {
                    return false;
                }
                self.status = BehaviorStatus::Running;
                true
            }
            BehaviorKind::Gate => {
                self.status = BehaviorStatus::Running;
                self.hooks.start(tick, context, timestamp);
                true
            }
        }
    }

    fn tick_or_stop(
        &mut self,
        tick: &mut BrainTick<'_>,
        context: &mut BrainTickContext,
        timestamp: i64,
    ) {
        match self.kind {
            BehaviorKind::OneShot => self.do_stop(tick, context, timestamp),
            BehaviorKind::Gate => {
                self.hooks.tick(tick, context, timestamp);
                if self.hooks.stops_after_tick() {
                    self.do_stop(tick, context, timestamp);
                }
            }
            BehaviorKind::Timed
                if !self.hooks.timed_out(timestamp, self.end_timestamp)
                    && self.hooks.can_still_use(tick, context, timestamp) =>
            {
                self.hooks.tick(tick, context, timestamp);
                if self.hooks.stops_after_tick() {
                    self.do_stop(tick, context, timestamp);
                }
            }
            BehaviorKind::Timed => {
                self.do_stop(tick, context, timestamp);
            }
        }
    }

    fn do_stop(
        &mut self,
        tick: &mut BrainTick<'_>,
        context: &mut BrainTickContext,
        timestamp: i64,
    ) {
        self.status = BehaviorStatus::Stopped;
        if self.kind != BehaviorKind::OneShot {
            self.hooks.stop(tick, context, timestamp);
        }
    }
}

impl Clone for Behavior {
    fn clone(&self) -> Self {
        Self {
            entry_condition: self.entry_condition.clone(),
            kind: self.kind,
            status: self.status,
            end_timestamp: self.end_timestamp,
            min_duration: self.min_duration,
            max_duration: self.max_duration,
            hooks: self.hooks.clone(),
        }
    }
}

// `Brain`.
pub struct Brain {
    memories: FxHashMap<MemoryModuleType, MemorySlot>,
    /// `Brain.sensors`: a `LinkedHashMap`, so the tick walks
    /// the order the provider named its sensor types in, and a re-registration
    /// of a type keeps that type's position.
    sensors: Vec<SensorEntry>,
    available_behaviors: BTreeMap<(i32, Activity), Vec<Behavior>>,
    has_schedule: bool,
    activity_requirements: FxHashMap<Activity, &'static [(MemoryModuleType, MemoryStatus)]>,
    activity_memories_to_erase_when_stopped: FxHashMap<Activity, &'static [MemoryModuleType]>,
    activity: Activity,
    last_schedule_update: i64,
}

impl Clone for Brain {
    fn clone(&self) -> Self {
        Self {
            memories: self.memories.clone(),
            sensors: self.sensors.clone(),
            available_behaviors: self.available_behaviors.clone(),
            has_schedule: self.has_schedule,
            activity_requirements: self.activity_requirements.clone(),
            activity_memories_to_erase_when_stopped: self
                .activity_memories_to_erase_when_stopped
                .clone(),
            activity: self.activity,
            last_schedule_update: self.last_schedule_update,
        }
    }
}

impl Default for Brain {
    fn default() -> Self {
        Self::new()
    }
}

impl Brain {
    const SCHEDULE_UPDATE_DELAY: i64 = 20;
    const NEVER_SCHEDULED: i64 = -9999;

    pub fn new() -> Self {
        Self {
            memories: FxHashMap::default(),
            sensors: Vec::new(),
            available_behaviors: BTreeMap::new(),
            has_schedule: false,
            activity_requirements: FxHashMap::default(),
            activity_memories_to_erase_when_stopped: FxHashMap::default(),
            activity: Activity::IDLE,
            last_schedule_update: Self::NEVER_SCHEDULED,
        }
    }

    pub fn register_memory(&mut self, memory_type: MemoryModuleType) {
        self.memories
            .entry(memory_type)
            .or_insert_with(MemorySlot::create);
    }

    /// Registers `sensor` and its own `requires` memories.
    /// Every sensor scans on the brain's first tick; Vanilla's random start
    /// delay only staggers scan phases.
    pub fn register_sensor(&mut self, sensor: SensorType) {
        for memory_type in sensor.requires() {
            self.register_memory(*memory_type);
        }
        let entry = SensorEntry {
            sensor_type: sensor,
            time_to_tick: 0,
        };
        // `Brain.sensors.put`: a type already registered
        // keeps its position and takes the new sensor.
        match self
            .sensors
            .iter_mut()
            .find(|existing| existing.sensor_type == sensor)
        {
            Some(existing) => *existing = entry,
            None => self.sensors.push(entry),
        }
    }

    pub fn add_activity(
        &mut self,
        activity: Activity,
        behaviors: impl IntoIterator<Item = (i32, Behavior)>,
        conditions: &'static [(MemoryModuleType, MemoryStatus)],
        memories_to_erase_when_stopped: &'static [MemoryModuleType],
    ) {
        self.activity_requirements.insert(activity, conditions);
        if !memories_to_erase_when_stopped.is_empty() {
            self.activity_memories_to_erase_when_stopped
                .insert(activity, memories_to_erase_when_stopped);
        }
        for (priority, behavior) in behaviors {
            behavior.for_each_required_memory(&mut |memory_type| self.register_memory(memory_type));
            self.available_behaviors
                .entry((priority, activity))
                .or_default()
                .push(behavior);
        }
    }

    pub fn set_schedule(&mut self, enabled: bool) {
        self.has_schedule = enabled;
    }

    pub fn use_default_activity(&mut self) {
        self.set_active_activity(Activity::IDLE);
    }

    pub fn is_active(&self, activity: Activity) -> bool {
        activity == Activity::CORE || activity == self.activity
    }

    pub fn set_active_activity_if_possible(&mut self, activity: Activity) {
        if self.activity_requirements_are_met(activity) {
            self.set_active_activity(activity);
        } else {
            self.use_default_activity();
        }
    }

    pub fn set_active_activity_to_first_valid(&mut self, activities: &[Activity]) {
        for activity in activities {
            if self.activity_requirements_are_met(*activity) {
                self.set_active_activity(*activity);
                break;
            }
        }
    }

    /// The schedule pass a freshly built brain takes before its first tick
    /// (`Villager.registerBrainGoals`).
    pub fn start_from_schedule(&mut self, game_time: i64, scheduled: Activity) {
        if self.last_schedule_update == Self::NEVER_SCHEDULED {
            self.update_activity_from_schedule(game_time, scheduled);
        }
    }

    // `Brain.updateActivityFromSchedule`.
    pub fn update_activity_from_schedule(&mut self, game_time: i64, scheduled: Activity) {
        if game_time - self.last_schedule_update <= Self::SCHEDULE_UPDATE_DELAY {
            return;
        }
        self.last_schedule_update = game_time;
        let scheduled = if self.has_schedule {
            scheduled
        } else {
            Activity::IDLE
        };
        if !self.is_active(scheduled) {
            self.set_active_activity_if_possible(scheduled);
        }
    }

    pub fn set_memory(&mut self, memory_type: MemoryModuleType, value: Option<MemoryValue>) {
        set_memory_internal(&mut self.memories, memory_type, value, None);
    }

    pub fn set_memory_with_expiry(
        &mut self,
        memory_type: MemoryModuleType,
        value: MemoryValue,
        time_to_live: i64,
    ) {
        set_memory_internal(
            &mut self.memories,
            memory_type,
            Some(value),
            Some(time_to_live),
        );
    }

    pub fn erase_memory(&mut self, memory_type: MemoryModuleType) {
        if let Some(slot) = self.memories.get_mut(&memory_type) {
            slot.clear();
        }
    }

    pub fn clear_memories(&mut self) {
        for slot in self.memories.values_mut() {
            slot.clear();
        }
    }

    pub fn memory(&self, memory_type: MemoryModuleType) -> Option<&MemoryValue> {
        memory_slot(&self.memories, memory_type).value()
    }

    pub fn has_memory_value(&self, memory_type: MemoryModuleType) -> bool {
        self.check_memory(memory_type, MemoryStatus::ValuePresent)
    }

    pub fn time_until_expiry(&self, memory_type: MemoryModuleType) -> i64 {
        memory_slot(&self.memories, memory_type).time_to_live()
    }

    pub fn check_memory(&self, memory_type: MemoryModuleType, status: MemoryStatus) -> bool {
        check_memory(&self.memories, memory_type, status)
    }

    pub fn apply_packed(&mut self, packed: &BrainPacked) {
        for memory in &packed.memories {
            set_memory_internal(
                &mut self.memories,
                memory.memory_type,
                Some(memory.value.clone()),
                memory.time_to_live,
            );
        }
    }

    pub fn pack(&self) -> BrainPacked {
        let mut memories = Vec::new();
        for (memory_type, slot) in &self.memories {
            if !memory_type.can_serialize() {
                continue;
            }
            let Some(value) = slot.value() else {
                continue;
            };
            memories.push(PackedMemory {
                memory_type: *memory_type,
                value: value.clone(),
                time_to_live: slot.can_expire().then_some(slot.time_to_live()),
            });
        }
        memories.sort_by_key(|memory| memory.memory_type.resource_id());
        BrainPacked { memories }
    }

    /// `Brain.getActiveActivities`, in a stable order:
    /// CORE, then the chosen activity.
    pub fn active_activities(&self) -> Vec<Activity> {
        vec![Activity::CORE, self.activity]
    }

    /// `Brain.forEach`: every registered slot, the empty
    /// ones included. The last argument is `LearningSlot.canExpire`, which is
    /// what a dump marks a line with.
    pub fn for_each_memory(
        &self,
        mut visit: impl FnMut(MemoryModuleType, Option<&MemoryValue>, bool),
    ) {
        for (memory_type, slot) in &self.memories {
            visit(*memory_type, slot.value(), slot.can_expire());
        }
    }

    pub fn for_each_memory_with_ttl(
        &self,
        mut visit: impl FnMut(MemoryModuleType, Option<&MemoryValue>, Option<i64>),
    ) {
        for (memory_type, slot) in &self.memories {
            visit(
                *memory_type,
                slot.value(),
                slot.can_expire().then_some(slot.time_to_live()),
            );
        }
    }

    /// The priorities and names of one activity's registered behaviours, which
    /// is what a provider's `ActivityData` literals pin.
    #[cfg(test)]
    pub fn behavior_priorities(&self, activity: Activity) -> Vec<(i32, &'static str)> {
        let mut rows = Vec::new();
        for ((priority, registered_activity), behaviors) in &self.available_behaviors {
            if *registered_activity != activity {
                continue;
            }
            for behavior in behaviors {
                rows.push((*priority, behavior.debug_string()));
            }
        }
        rows
    }

    pub fn running_behaviors(&self) -> Vec<&'static str> {
        self.available_behaviors
            .values()
            .flatten()
            .filter(|behavior| behavior.status() == BehaviorStatus::Running)
            .map(Behavior::debug_string)
            .collect()
    }

    // `Brain.tick`.
    pub fn tick(&mut self, context: &mut BrainTickContext, world: &mut dyn BrainWorld) {
        self.forget_outdated_memories();
        self.tick_sensors(world);
        self.start_each_non_running_behavior(context, world);
        self.tick_each_running_behavior(context, world);
    }

    fn forget_outdated_memories(&mut self) {
        for slot in self.memories.values_mut() {
            slot.tick();
        }
    }

    // `Brain.tickSensors` over `Sensor.tick`
    // the counter is decremented first, a scan that
    // comes due is reset to the rate, and an expired counter keeps the sensor
    // answered on every later tick.
    fn tick_sensors(&mut self, world: &mut dyn BrainWorld) {
        let mut tick = BrainTick::new(&mut self.memories, self.activity, world);
        for entry in &mut self.sensors {
            entry.time_to_tick -= 1;
            if entry.time_to_tick <= 0 {
                let scan_rate = entry.sensor_type.scan_rate();
                entry.time_to_tick = i64::from(scan_rate);
                entry.sensor_type.do_tick(&mut tick);
            }
        }
    }

    fn start_each_non_running_behavior(
        &mut self,
        context: &mut BrainTickContext,
        world: &mut dyn BrainWorld,
    ) {
        let timestamp = context.game_time();
        let mut tick = BrainTick::new(&mut self.memories, self.activity, world);
        for ((_, activity), behaviors) in &mut self.available_behaviors {
            if !tick.is_active(*activity) {
                continue;
            }
            for behavior in behaviors {
                if behavior.status() == BehaviorStatus::Stopped {
                    behavior.try_start(&mut tick, context, timestamp);
                }
            }
        }
    }

    fn tick_each_running_behavior(
        &mut self,
        context: &mut BrainTickContext,
        world: &mut dyn BrainWorld,
    ) {
        let timestamp = context.game_time();
        let mut tick = BrainTick::new(&mut self.memories, self.activity, world);
        for behavior in self.available_behaviors.values_mut().flatten() {
            if behavior.status() == BehaviorStatus::Running {
                behavior.tick_or_stop(&mut tick, context, timestamp);
            }
        }
    }

    fn set_active_activity(&mut self, activity: Activity) {
        if self.is_active(activity) {
            return;
        }
        self.erase_memories_for_other_activities_than(activity);
        self.activity = activity;
    }

    fn erase_memories_for_other_activities_than(&mut self, activity: Activity) {
        for old in [Activity::CORE, self.activity] {
            if old == activity {
                continue;
            }
            for memory_type in self
                .activity_memories_to_erase_when_stopped
                .get(&old)
                .copied()
                .unwrap_or_default()
            {
                set_memory_internal(&mut self.memories, *memory_type, None, None);
            }
        }
    }

    fn activity_requirements_are_met(&self, activity: Activity) -> bool {
        let Some(requirements) = self.activity_requirements.get(&activity) else {
            return false;
        };
        requirements
            .iter()
            .all(|(memory_type, status)| self.check_memory(*memory_type, *status))
    }
}

#[cfg(test)]
pub(crate) mod tests;
