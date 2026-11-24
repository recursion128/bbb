//! The behaviour bodies of `net.minecraft.world.entity.ai.behavior` this
//! runtime runs.

use crate::random::GameRandom;
use bbb_protocol::packets::{BlockPos, Vec3d};

mod animal_make_love;
mod frog_idle;
mod frog_land;
mod long_jump;
mod villager_alarm;
mod villager_life;
mod villager_play;
mod villager_rest;
mod villager_stroll;
mod villager_trades;
mod villager_work;

pub use animal_make_love::{animal_make_love, AnimalMakeLove};
pub use frog_idle::{croak, trigger_if_in_water, trigger_if_on_ground, Croak, TriggerIfOnGround};
pub use frog_land::{
    try_find_land, try_find_land_near_water, try_lay_spawn_on_fluid_near_land, TryFindLand,
    TryFindLandNearWater, TryLaySpawnOnFluidNearLand,
};
pub use long_jump::{
    long_jump_mid_jump, long_jump_to_preferred_block, LongJumpMidJump, LongJumpToPreferredBlock,
};
pub use villager_alarm::{
    locate_hiding_place, set_hidden_state, set_walk_target_away_from, villager_calm_down,
    villager_panic_trigger,
};
pub use villager_life::{
    harvest_farmland, interact_with_cat, interact_with_villager, socialize_at_bell,
    stroll_around_poi, trade_with_villager, use_bonemeal, villager_make_love,
};
pub use villager_play::{
    jump_on_bed, play_tag_with_other_kids, villager_full_look, villager_minimal_look,
    villager_play_fallback,
};
pub use villager_rest::{
    inside_brownian_walk, set_closest_home_as_walk_target, sleep_in_bed, wake_up,
};
pub use villager_stroll::{
    go_to_closest_village, move_to_sky_seeing_spot, village_bound_random_stroll,
    village_bound_random_stroll_within,
};
pub use villager_trades::{set_look_and_interact_player, show_trades_to_player};
pub use villager_work::{
    acquire_poi, assign_profession_from_job_site, go_to_potential_job_site,
    look_and_follow_trading_player, poi_competitor_scan, reset_profession,
    set_walk_target_from_block_memory, stroll_to_poi, stroll_to_poi_list, validate_nearby_poi,
    work_at_poi, yield_job_site,
};

use super::effect::BrainWorldEffect;
use super::memory::{
    MemoryModuleType, MemoryPositionTracker, MemoryStatus, MemoryValue, MemoryWalkTarget,
};
use super::sensors::is_entity_targetable;
use super::{Behavior, BehaviorHooks, BehaviorStatus, BrainTick, BrainTickContext};
use crate::entity::ai::mob::frog;

/// `DamageTypeTags.PANIC_CAUSES`'s registry tag, which `AnimalPanic`'s
/// default `panicCausingDamageTypes` reads (`AnimalPanic.java:36`).
pub const PANIC_CAUSES_TAG: &str = "minecraft:panic_causes";

/// `Pose.CROAKING.getId()` (`EntityPose.java`), which
/// `ShootTongue.checkExtraStartConditions` refuses (`ShootTongue.java:61`).
const POSE_CROAKING: i32 = bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROAKING_ID;
/// `Pose.STANDING.getId()`, which `ShootTongue.stop` restores
/// (`ShootTongue.java:82`).
const POSE_STANDING: i32 = bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_STANDING_ID;
/// `Pose.USING_TONGUE.getId()`, which `ShootTongue.tick` sets on the catch
/// (`ShootTongue.java:106`).
const POSE_USING_TONGUE: i32 = bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_USING_TONGUE_ID;

/// `AnimalPanic`.
///
/// Wired: the entry condition, `checkExtraStartConditions`,
/// `canStillUse`, `start`, `stop` and `tick`
/// with its `getPanicPos`.
#[derive(Debug, Clone, Copy)]
pub struct AnimalPanic {
    /// `AnimalPanic.speedMultiplier` (`:31`), which `tick` writes into the
    /// `WALK_TARGET` it sets (`:84`).
    speed_modifier: f32,
}

/// `AnimalPanic.tick`.
impl AnimalPanic {
    /// `AnimalPanic.getPanicPos`.
    ///
    /// `None` is a level that cannot answer one of the arms' reads, which
    /// refuses the whole position: Vanilla has no unreadable-block branch, and
    /// falling through to the position getter instead would walk a burning
    /// body the wrong way. `Some(None)` is the null position `tick` refuses to
    /// write.
    fn panic_pos(
        &self,
        tick: &mut BrainTick<'_>,
        context: &mut BrainTickContext,
    ) -> Option<Option<Vec3d>> {
        if tick.world().body().on_fire {
            // `Mth.ceil(mob.getBbWidth()) == 2` (`:107`) over the body's own
            // bounding box, which is the box `getDimensions(getPose())` gives.
            let bounding_box = tick.world().body().bounding_box;
            let two_wide = (bounding_box.max_x - bounding_box.min_x).ceil() == 2.0;
            if let Some(water) = tick.world_mut().body_closest_water(5, 1, two_wide)? {
                // `lookForWater(level, body).map(Vec3::atBottomCenterOf)`
                // (`:91`), which the position getter is never reached for.
                return Some(Some(Vec3d {
                    x: f64::from(water.x) + 0.5,
                    y: f64::from(water.y),
                    z: f64::from(water.z) + 0.5,
                }));
            }
        }
        // `this.positionGetter.apply(body)`, which the frog's constructor
        // fills with `LandRandomPos.getPos(mob, 5, 4)` (`:36`).
        tick.world_mut()
            .body_land_random_pos(context.body_random(), 5, 4)
    }
}

impl BehaviorHooks for AnimalPanic {
    // `AnimalPanic.checkExtraStartConditions`:
    // `brain.getMemory(HURT_BY).map(d -> d.is(panicCausingDamageTypes))`
    // or `brain.hasMemoryValue(IS_PANICKING)`.
    fn extra_start_conditions(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
    ) -> bool {
        let panics = match tick.memory(MemoryModuleType::HurtBy) {
            Some(MemoryValue::DamageSource(source)) => tick
                .world()
                .damage_type_in_tag(&source.damage_type, PANIC_CAUSES_TAG),
            _ => false,
        };
        panics || tick.has_memory_value(MemoryModuleType::IsPanicking)
    }

    // `AnimalPanic.canStillUse`.
    fn can_still_use(
        &self,
        _tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        true
    }

    // `AnimalPanic.start`.
    fn start(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) {
        tick.set_memory_value(MemoryModuleType::IsPanicking, MemoryValue::Bool(true));
        tick.erase_memory(MemoryModuleType::WalkTarget);
        tick.world_mut().stop_body_navigation();
    }

    // `AnimalPanic.tick`: a new destination only
    // while the navigation has nothing to follow, and never a null one.
    fn tick(&mut self, tick: &mut BrainTick<'_>, context: &mut BrainTickContext, _timestamp: i64) {
        if !tick.world().body_navigation_is_done() {
            return;
        }
        let Some(position) = self.panic_pos(tick, context) else {
            return;
        };
        let Some(position) = position else {
            return;
        };
        tick.set_memory_value(
            MemoryModuleType::WalkTarget,
            MemoryValue::WalkTarget(make_walk_target(position, self.speed_modifier, 0)),
        );
    }

    // `AnimalPanic.stop`.
    fn stop(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, _timestamp: i64) {
        tick.erase_memory(MemoryModuleType::IsPanicking);
    }

    fn debug_string(&self) -> &'static str {
        "AnimalPanic"
    }
}

/// `AnimalPanic`'s behaviour, as the frog registers it
/// (`FrogAi`, `new AnimalPanic<>(2.0F)`).
pub fn animal_panic(speed_modifier: f32) -> Behavior {
    // `super(Map.of(IS_PANICKING, REGISTERED, HURT_BY, REGISTERED), 100, 120)`
    Behavior::with_duration(
        vec![
            (MemoryModuleType::IsPanicking, MemoryStatus::Registered),
            (MemoryModuleType::HurtBy, MemoryStatus::Registered),
        ],
        100,
        120,
        AnimalPanic { speed_modifier },
    )
}

/// `CountDownCooldownTicks`.
#[derive(Debug, Clone, Copy)]
pub struct CountDownCooldownTicks {
    cooldown_ticks: MemoryModuleType,
}

impl BehaviorHooks for CountDownCooldownTicks {
    // `CountDownCooldownTicks.timedOut`:
    // the duration is refused, so only the memory ending stops it.
    fn timed_out(&self, _timestamp: i64, _end_timestamp: i64) -> bool {
        false
    }

    // `CountDownCooldownTicks.canStillUse` (`:27-31`).
    fn can_still_use(
        &self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        matches!(
            tick.memory(self.cooldown_ticks),
            Some(MemoryValue::Int(ticks)) if *ticks > 0
        )
    }

    // `CountDownCooldownTicks.tick` (`:33-37`).
    fn tick(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, _timestamp: i64) {
        if let Some(MemoryValue::Int(ticks)) = tick.memory(self.cooldown_ticks) {
            let next = *ticks - 1;
            tick.set_memory_value(self.cooldown_ticks, MemoryValue::Int(next));
        }
    }

    // `CountDownCooldownTicks.stop` (`:39-42`).
    fn stop(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, _timestamp: i64) {
        tick.erase_memory(self.cooldown_ticks);
    }

    fn debug_string(&self) -> &'static str {
        "CountDownCooldownTicks"
    }
}

/// `new CountDownCooldownTicks(cooldownTicks)`
pub fn count_down_cooldown_ticks(cooldown_ticks: MemoryModuleType) -> Behavior {
    Behavior::new(
        vec![(cooldown_ticks, MemoryStatus::ValuePresent)],
        CountDownCooldownTicks { cooldown_ticks },
    )
}

/// `StartAttacking`. `BehaviorBuilder.create`
/// returns a `OneShot`, whose whole body is one
/// trigger, so the two hooks below are that trigger's two halves and it never
/// survives its own tick.
#[derive(Debug, Clone, Copy)]
pub struct StartAttacking {
    /// `StartAttacking.create`'s `canAttackPredicate` (`:21-23`).
    can_attack: StartAttackingPredicate,
    /// `StartAttacking.create`'s `targetFinderFunction` (`:25-28`), read as
    /// the memory it resolves its target from.
    finder: MemoryModuleType,
}

/// The two `canAttackPredicate`s the providers this runtime builds pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartAttackingPredicate {
    /// `!BehaviorUtils.isBreeding(mob)`, which `FrogAi` passes
    /// a body carrying a breeding target is
    /// refused.
    NotBreeding,
    /// `mob instanceof Creaking creaking && creaking.isActive()`, which
    /// `CreakingAi` passes.
    BodyIsActive,
}

impl StartAttackingPredicate {
    fn test(self, tick: &BrainTick<'_>) -> bool {
        match self {
            Self::NotBreeding => !tick.has_memory_value(MemoryModuleType::BreedTarget),
            Self::BodyIsActive => tick.world().body().is_active,
        }
    }
}

impl BehaviorHooks for StartAttacking {
    // `StartAttacking.create`'s lambda.
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        // `canAttackPredicate.test(level, body)` (`:21-23`).
        if !self.can_attack.test(tick) {
            return false;
        }
        // `targetFinderFunction.get(level, body)` (`:25-28`).
        let Some(target) = tick.memory_entity(self.finder) else {
            return false;
        };
        // `body.canAttack(targetEntity)`.
        if !tick.world().body_can_attack(target) {
            return false;
        }
        tick.set_memory_value(
            MemoryModuleType::AttackTarget,
            MemoryValue::EntityRef(target),
        );
        tick.erase_memory(MemoryModuleType::CantReachWalkTargetSince);
        true
    }

    fn debug_string(&self) -> &'static str {
        "StartAttacking"
    }
}

/// The `OneShot` `StartAttacking.create(...)` builds
/// its `BehaviorBuilder` group asks for an
/// absent `ATTACK_TARGET` and a registered
/// `CANT_REACH_WALK_TARGET_SINCE`.
fn start_attacking_with(can_attack: StartAttackingPredicate, finder: MemoryModuleType) -> Behavior {
    Behavior::one_shot(
        vec![
            (MemoryModuleType::AttackTarget, MemoryStatus::ValueAbsent),
            (
                MemoryModuleType::CantReachWalkTargetSince,
                MemoryStatus::Registered,
            ),
        ],
        StartAttacking { can_attack, finder },
    )
}

/// `StartAttacking.create((level, mob) -> !BehaviorUtils.isBreeding(mob),
/// (level, mob) -> mob.getBrain().getMemory(NEAREST_ATTACKABLE))`
pub fn start_attacking() -> Behavior {
    start_attacking_with(
        StartAttackingPredicate::NotBreeding,
        MemoryModuleType::NearestAttackable,
    )
}

/// `StartAttacking.create((level, mob) -> mob instanceof Creaking creaking &&
/// creaking.isActive(), (level, mob) ->
/// mob.getBrain().getMemory(NEAREST_VISIBLE_ATTACKABLE_PLAYER))`
/// the IDLE behaviour that re-acquires a player the
/// protector lost while it stayed active.
pub fn start_attacking_when_active() -> Behavior {
    start_attacking_with(
        StartAttackingPredicate::BodyIsActive,
        MemoryModuleType::NearestVisibleAttackablePlayer,
    )
}

/// `StopAttackingIfTargetInvalid`,
/// another `OneShot`, which the frog registers through its no-argument factory
#[derive(Debug, Clone, Copy, Default)]
pub struct StopAttackingIfTargetInvalid {
    /// `CreakingAi.isAttackTargetStillReachable`
    /// reads `NEAREST_VISIBLE_ATTACKABLE_PLAYERS`; the default condition reads
    /// nothing.
    still_reachable_in: Option<MemoryModuleType>,
}

impl BehaviorHooks for StopAttackingIfTargetInvalid {
    // `StopAttackingIfTargetInvalid.create`'s lambda
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        let Some(target) = tick.memory_entity(MemoryModuleType::AttackTarget) else {
            return false;
        };
        if !target_is_still_valid(tick, target) || self.stop_attacking_when(tick, target) {
            tick.erase_memory(MemoryModuleType::AttackTarget);
        }
        // The lambda answers true on both arms (`:41`, `:46`).
        true
    }

    fn debug_string(&self) -> &'static str {
        "StopAttackingIfTargetInvalid"
    }
}

impl StopAttackingIfTargetInvalid {
    /// The `stopAttackingWhen` condition,
    /// whose negation is the `!` the caller's lambda applies.
    fn stop_attacking_when(&self, tick: &BrainTick<'_>, target: i32) -> bool {
        let Some(memory_type) = self.still_reachable_in else {
            return false;
        };
        match tick.memory(memory_type) {
            Some(MemoryValue::EntityList(entities)) => !entities.contains(&target),
            _ => true,
        }
    }
}

/// `body.canAttack(target) && (!canGrowTiredOfTryingToReachTarget ||
/// !isTiredOfTryingToReachTarget(body, cantReachSince)) && target.isAlive() &&
/// target.level() == body.level() && !stopAttackingWhen.test(level, target)`
///
/// The default `stopAttackingWhen` answers false and
/// `isTiredOfTryingToReachTarget` compares the level's game time against
/// `CANT_REACH_WALK_TARGET_SINCE`. A target on another level, or
/// one this world no longer holds, answers `false` at `entity_facts`.
fn target_is_still_valid(tick: &BrainTick<'_>, target: i32) -> bool {
    let Some(facts) = tick.world().entity_facts(target) else {
        return false;
    };
    if !facts.alive || !tick.world().body_can_attack(target) {
        return false;
    }
    match tick.memory(MemoryModuleType::CantReachWalkTargetSince) {
        Some(MemoryValue::Long(since)) => tick.world().game_time() - since <= 200,
        _ => true,
    }
}

/// The `OneShot` `StopAttackingIfTargetInvalid.create()` builds
pub fn stop_attacking_if_target_invalid() -> Behavior {
    stop_attacking_if_target_invalid_unless_in(None)
}

/// `StopAttackingIfTargetInvalid.create(stopAttackingWhen)`
/// whose extra condition answers
/// true for a target the named list memory does not hold.
pub fn stop_attacking_if_target_invalid_unless_in(
    still_reachable_in: Option<MemoryModuleType>,
) -> Behavior {
    Behavior::one_shot(
        vec![
            (MemoryModuleType::AttackTarget, MemoryStatus::ValuePresent),
            (
                MemoryModuleType::CantReachWalkTargetSince,
                MemoryStatus::Registered,
            ),
        ],
        StopAttackingIfTargetInvalid { still_reachable_in },
    )
}

/// `SetWalkTargetFromAttackTargetIfTargetOutOfReach`
/// a `OneShot`.
#[derive(Debug, Clone, Copy)]
pub struct SetWalkTargetFromAttackTargetIfTargetOutOfReach {
    speed_modifier: f32,
}

impl BehaviorHooks for SetWalkTargetFromAttackTargetIfTargetOutOfReach {
    // `SetWalkTargetFromAttackTargetIfTargetOutOfReach.create`'s trigger
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        let Some(target) = tick.memory_entity(MemoryModuleType::AttackTarget) else {
            return false;
        };
        // `NearestVisibleLivingEntities.contains(toAttack)`: an absent memory
        // answers false here too, which is what `i.tryGet(...).isPresent()`
        // would have made the whole conjunction answer.
        let visible = body_sees_entity(tick, target);
        if visible && is_within_attack_range(tick, target) {
            // `walkTarget.erase()` (`:32`).
            tick.erase_memory(MemoryModuleType::WalkTarget);
            return true;
        }
        // `lookTarget.set(new EntityTracker(toAttack, true))` (`:34`), which
        // `LookAtTargetSink` resolves live.
        tick.set_memory_value(
            MemoryModuleType::LookTarget,
            MemoryValue::EntityTracker(target),
        );
        // `walkTarget.set(new WalkTarget(new EntityTracker(toAttack, false),
        // speedModifier.apply(body), 0))` (`:35`). A target this world cannot
        // place leaves the memory as it was.
        if let Some(tracker) = entity_tracker(tick, target) {
            tick.set_memory_value(
                MemoryModuleType::WalkTarget,
                MemoryValue::WalkTarget(MemoryWalkTarget {
                    target: tracker,
                    speed_modifier: self.speed_modifier,
                    close_enough_dist: 0,
                }),
            );
        }
        true
    }

    fn debug_string(&self) -> &'static str {
        "SetWalkTargetFromAttackTargetIfTargetOutOfReach"
    }
}

/// The `OneShot` `SetWalkTargetFromAttackTargetIfTargetOutOfReach.create(float)`
/// builds, whose
/// `BehaviorBuilder` group asks for a registered `WALK_TARGET`, a registered
/// `LOOK_TARGET`, a present `ATTACK_TARGET` and a registered
/// `NEAREST_VISIBLE_LIVING_ENTITIES`.
pub fn set_walk_target_from_attack_target_if_target_out_of_reach(speed_modifier: f32) -> Behavior {
    Behavior::one_shot(
        vec![
            (MemoryModuleType::WalkTarget, MemoryStatus::Registered),
            (MemoryModuleType::LookTarget, MemoryStatus::Registered),
            (MemoryModuleType::AttackTarget, MemoryStatus::ValuePresent),
            (
                MemoryModuleType::NearestVisibleLivingEntities,
                MemoryStatus::Registered,
            ),
        ],
        SetWalkTargetFromAttackTargetIfTargetOutOfReach { speed_modifier },
    )
}

/// `BehaviorUtils.isWithinAttackRange` for a
/// body whose main hand holds no usable ranged weapon -- which is
/// `Mob.canUseNonMeleeWeapon`'s default answer -- so the
/// melee branch is `Mob.isWithinMeleeAttackRange`.
fn is_within_attack_range(tick: &BrainTick<'_>, target: i32) -> bool {
    let Some(facts) = tick.world().entity_facts(target) else {
        return false;
    };
    let body = tick.world().body();
    match body.melee_reach {
        Some(reach) => reach.reaches(facts.bounding_box),
        None => crate::entity::ai::mob::melee_attack_box(body.bounding_box)
            .intersects(facts.bounding_box),
    }
}

/// `ShootTongue` with the frog's own two sounds
#[derive(Debug, Clone, Copy)]
pub struct ShootTongue {
    state: TongueState,
    eat_animation_timer: i32,
    calculate_path_counter: i32,
}

/// `ShootTongue.State`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TongueState {
    MoveToTarget,
    CatchAnimation,
    EatAnimation,
    Done,
}

impl Default for ShootTongue {
    fn default() -> Self {
        Self {
            state: TongueState::Done,
            eat_animation_timer: 0,
            calculate_path_counter: 0,
        }
    }
}

impl ShootTongue {
    /// `ShootTongue.TIME_OUT_DURATION` (`ShootTongue.java:22`).
    pub const TIME_OUT_DURATION: i32 = 100;
    /// `ShootTongue.CATCH_ANIMATION_DURATION` (`ShootTongue.java:23`).
    pub const CATCH_ANIMATION_DURATION: i32 = 6;
    /// `ShootTongue.TONGUE_ANIMATION_DURATION` (`ShootTongue.java:24`).
    pub const TONGUE_ANIMATION_DURATION: i32 = 10;
    /// `ShootTongue.EATING_DISTANCE` (`ShootTongue.java:25`).
    pub const EATING_DISTANCE: f32 = 1.75;
    /// `ShootTongue.EATING_MOVEMENT_FACTOR` (`ShootTongue.java:26`).
    pub const EATING_MOVEMENT_FACTOR: f64 = 0.75;
    /// `ShootTongue.UNREACHABLE_TONGUE_TARGETS_COOLDOWN_DURATION`
    /// (`ShootTongue.java:27`).
    pub const UNREACHABLE_COOLDOWN: i64 = 100;
    /// `ShootTongue.MAX_UNREACHBLE_TONGUE_TARGETS_IN_MEMORY`
    /// (`ShootTongue.java:28`).
    pub const MAX_UNREACHABLE_TARGETS: usize = 5;
    /// `SoundEvents.FROG_TONGUE` (`FrogAi.java:201`).
    pub const TONGUE_SOUND: &'static str = "minecraft:entity.frog.tongue";
    /// `SoundEvents.FROG_EAT` (`FrogAi.java:201`).
    pub const EAT_SOUND: &'static str = "minecraft:entity.frog.eat";
    /// `Mob.doHurtTarget`'s `attacker.damageSources().mobAttack(this)`
    /// (`Mob.java:1358`).
    pub const ATTACK_DAMAGE_TYPE: &'static str = "minecraft:mob_attack";
}

impl BehaviorHooks for ShootTongue {
    // `ShootTongue.checkExtraStartConditions`.
    fn extra_start_conditions(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
    ) -> bool {
        let Some(target) = tick.memory_entity(MemoryModuleType::AttackTarget) else {
            return false;
        };
        if !can_pathfind_to_target(tick, target) {
            tick.erase_memory(MemoryModuleType::AttackTarget);
            remember_unreachable_target(tick, target);
            return false;
        }
        tick.world().body().pose_id != POSE_CROAKING
            && tick
                .world()
                .entity_facts(target)
                .is_some_and(|facts| frog::can_eat(tick.world(), &facts))
    }

    // `ShootTongue.canStillUse`.
    fn can_still_use(
        &self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        tick.has_memory_value(MemoryModuleType::AttackTarget)
            && self.state != TongueState::Done
            && !tick.has_memory_value(MemoryModuleType::IsPanicking)
    }

    // `ShootTongue.start`.
    fn start(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) {
        let Some(target) = tick.memory_entity(MemoryModuleType::AttackTarget) else {
            return;
        };
        // `BehaviorUtils.lookAtEntity(body, target)`.
        tick.set_memory_value(
            MemoryModuleType::LookTarget,
            MemoryValue::EntityTracker(target),
        );
        tick.world_mut().set_tongue_target(Some(target));
        let Some(position) = entity_position(tick, target) else {
            return;
        };
        tick.set_memory_value(
            MemoryModuleType::WalkTarget,
            MemoryValue::WalkTarget(make_walk_target(position, 2.0, 0)),
        );
        self.calculate_path_counter = 10;
        self.state = TongueState::MoveToTarget;
    }

    // `ShootTongue.stop`.
    fn stop(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, _timestamp: i64) {
        tick.erase_memory(MemoryModuleType::AttackTarget);
        tick.world_mut().set_tongue_target(None);
        tick.world_mut().queue_effect(BrainWorldEffect::SetPose {
            pose_id: POSE_STANDING,
        });
    }

    // `ShootTongue.tick`.
    fn tick(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, _timestamp: i64) {
        let Some(target) = tick.memory_entity(MemoryModuleType::AttackTarget) else {
            return;
        };
        tick.world_mut().set_tongue_target(Some(target));
        match self.state {
            // `case MOVE_TO_TARGET` (`:103-116`).
            TongueState::MoveToTarget => {
                let close = tick
                    .world()
                    .entity_distance_to_sqr(target)
                    .is_some_and(|distance| {
                        distance < f64::from(Self::EATING_DISTANCE * Self::EATING_DISTANCE)
                    });
                if close {
                    self.catch(tick, target);
                } else if self.calculate_path_counter <= 0 {
                    if let Some(position) = entity_position(tick, target) {
                        tick.set_memory_value(
                            MemoryModuleType::WalkTarget,
                            MemoryValue::WalkTarget(make_walk_target(position, 2.0, 0)),
                        );
                    }
                    self.calculate_path_counter = 10;
                } else {
                    self.calculate_path_counter -= 1;
                }
            }
            // `case CATCH_ANIMATION` (`:117-122`), whose post-increment runs
            // before the comparison.
            TongueState::CatchAnimation => {
                let elapsed = self.eat_animation_timer;
                self.eat_animation_timer += 1;
                if elapsed >= Self::CATCH_ANIMATION_DURATION {
                    self.state = TongueState::EatAnimation;
                    self.eat_entity(tick);
                }
            }
            // `case EAT_ANIMATION` (`:123-128`), which falls through into
            // `case DONE` (`:130`).
            TongueState::EatAnimation => {
                if self.eat_animation_timer >= Self::TONGUE_ANIMATION_DURATION {
                    self.state = TongueState::Done;
                } else {
                    self.eat_animation_timer += 1;
                }
            }
            TongueState::Done => {}
        }
    }

    fn debug_string(&self) -> &'static str {
        "ShootTongue"
    }
}

impl ShootTongue {
    // `ShootTongue.tick`'s MOVE_TO_TARGET arm at `target.distanceTo(body) <
    // 1.75F`.
    fn catch(&mut self, tick: &mut BrainTick<'_>, target: i32) {
        tick.world_mut()
            .queue_effect(BrainWorldEffect::PlayBodySound {
                resource_id: Self::TONGUE_SOUND,
                source: bbb_protocol::packets::SoundSource::Neutral,
                volume: 2.0,
                pitch: 1.0,
            });
        tick.world_mut().queue_effect(BrainWorldEffect::SetPose {
            pose_id: POSE_USING_TONGUE,
        });
        // `target.setDeltaMovement(target.position().vectorTo(body.position())
        // .normalize().scale(0.75))`.
        if let Some(movement) = pull_toward_body(tick, target) {
            tick.world_mut()
                .queue_effect(BrainWorldEffect::SetDeltaMovement {
                    entity_id: target,
                    movement,
                });
        }
        self.eat_animation_timer = 0;
        self.state = TongueState::CatchAnimation;
    }

    // `ShootTongue.eatEntity`: the eat sound
    // precedes the hurt, and a target the hurt kills is removed outright.
    fn eat_entity(&self, tick: &mut BrainTick<'_>) {
        tick.world_mut()
            .queue_effect(BrainWorldEffect::PlayBodySound {
                resource_id: Self::EAT_SOUND,
                source: bbb_protocol::packets::SoundSource::Neutral,
                volume: 2.0,
                pitch: 1.0,
            });
        let Some(target) = tick.world_mut().tongue_target() else {
            return;
        };
        if !entity_is_alive(tick, target) {
            return;
        }
        let amount = tick.world().body().attack_damage;
        // `body.doHurtTarget(level, target)` and the `if (!target.isAlive())
        // target.remove(KILLED)` behind it are one
        // effect: the store applies it after this tick, so the body cannot ask
        // whether the bite killed the food.
        tick.world_mut().queue_effect(BrainWorldEffect::HurtEntity {
            target_id: target,
            damage_type: Self::ATTACK_DAMAGE_TYPE,
            amount,
            remove_if_killed: true,
        });
    }
}

/// `new ShootTongue(tongueSound, eatSound)` (`FrogAi`).
pub fn shoot_tongue() -> Behavior {
    // `super(Map.of(WALK_TARGET, VALUE_ABSENT, LOOK_TARGET, REGISTERED,
    // ATTACK_TARGET, VALUE_PRESENT, IS_PANICKING, VALUE_ABSENT), 100)`
    Behavior::with_duration(
        vec![
            (MemoryModuleType::WalkTarget, MemoryStatus::ValueAbsent),
            (MemoryModuleType::LookTarget, MemoryStatus::Registered),
            (MemoryModuleType::AttackTarget, MemoryStatus::ValuePresent),
            (MemoryModuleType::IsPanicking, MemoryStatus::ValueAbsent),
        ],
        ShootTongue::TIME_OUT_DURATION,
        ShootTongue::TIME_OUT_DURATION,
        ShootTongue::default(),
    )
}

/// `LookAtTargetSink`, which `FrogAi`
/// registers as `new LookAtTargetSink(45, 90)`.
#[derive(Debug, Clone, Copy, Default)]
pub struct LookAtTargetSink;

impl BehaviorHooks for LookAtTargetSink {
    // `LookAtTargetSink.canStillUse`:
    // `getMemory(LOOK_TARGET).filter(pos -> pos.isVisibleBy(body)).isPresent()`.
    fn can_still_use(
        &self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        match tick.memory(MemoryModuleType::LookTarget) {
            Some(MemoryValue::EntityTracker(entity_id)) => {
                // `EntityTracker.isVisibleBy`: nonliving targets do not use the living sensor.
                tick.world().entity_facts(*entity_id).is_some_and(|facts| {
                    facts.alive && (!facts.living || body_sees_entity(tick, *entity_id))
                })
            }
            // `BlockPosTracker.isVisibleBy` answers true outright
            Some(MemoryValue::BlockPos(_)) => true,
            _ => false,
        }
    }

    // `LookAtTargetSink.stop` (`:18-20`).
    fn stop(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, _timestamp: i64) {
        tick.erase_memory(MemoryModuleType::LookTarget);
    }

    // `LookAtTargetSink.tick` (`:22-24`) over `PositionTracker.
    // currentPosition` (`EntityTracker`,
    // `BlockPosTracker`): the tracked entity's position plus its eye
    // height, or the tracked block's centre.
    fn tick(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, _timestamp: i64) {
        let Some(position) = look_target_aim_position(tick) else {
            return;
        };
        tick.world_mut().look_at(position);
    }

    fn debug_string(&self) -> &'static str {
        "LookAtTargetSink"
    }
}

/// `PositionTracker.currentPosition()` for the two kinds of tracker this crate
/// writes into `LOOK_TARGET`: the tracked entity's position plus its eye height
/// and the tracked block's centre
/// (`Vec3.atCenterOf`).
///
/// A `BlockPosTracker` is carried as the memory's own block position, because
/// its `PositionTracker` has no separate value kind in this crate; every
/// reader of `LOOK_TARGET` treats both spellings the same way.
fn look_target_aim_position(tick: &BrainTick<'_>) -> Option<Vec3d> {
    match tick.memory(MemoryModuleType::LookTarget) {
        Some(MemoryValue::EntityTracker(entity_id)) => {
            tick.world().entity_facts(*entity_id).map(|facts| Vec3d {
                y: facts.eye_y,
                ..facts.position
            })
        }
        Some(MemoryValue::BlockPos(pos)) => Some(Vec3d {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y) + 0.5,
            z: f64::from(pos.z) + 0.5,
        }),
        _ => None,
    }
}

/// The `PositionTracker` `LOOK_TARGET` holds, which this crate stores as the
/// tracked entity id (`EntityTracker`) or the tracked block
/// (`BlockPosTracker`).
fn look_target_tracker(tick: &BrainTick<'_>) -> Option<MemoryPositionTracker> {
    match tick.memory(MemoryModuleType::LookTarget) {
        Some(MemoryValue::EntityTracker(entity_id)) => entity_tracker(tick, *entity_id),
        Some(MemoryValue::BlockPos(pos)) => Some(MemoryPositionTracker::BlockPos(*pos)),
        _ => None,
    }
}

/// `new LookAtTargetSink(minDuration, maxDuration)`.
pub fn look_at_target_sink() -> Behavior {
    Behavior::with_duration(
        vec![(MemoryModuleType::LookTarget, MemoryStatus::ValuePresent)],
        45,
        90,
        LookAtTargetSink,
    )
}

/// `MoveToTargetSink`, which `FrogAi`
/// registers with its no-argument constructor.
///
/// `tryComputePath` keeps a non-null path even when it cannot reach the target;
/// only a null path samples a partial step.
#[derive(Debug, Clone, Copy, Default)]
pub struct MoveToTargetSink {
    remaining_cooldown: i32,
    path: Option<super::memory::MemoryPath>,
    speed_modifier: f32,
    last_target_pos: Option<BlockPos>,
}

impl BehaviorHooks for MoveToTargetSink {
    // `MoveToTargetSink.checkExtraStartConditions` (`:45-65`).
    fn extra_start_conditions(
        &mut self,
        tick: &mut BrainTick<'_>,
        context: &mut BrainTickContext,
    ) -> bool {
        if self.remaining_cooldown > 0 {
            self.remaining_cooldown -= 1;
            return false;
        }
        let Some(target) = walk_target(tick) else {
            return false;
        };
        let reached = reached_target(tick, target);
        if !reached {
            self.path = try_compute_path(tick, context, target);
            if self.path.is_some() {
                self.speed_modifier = target.speed_modifier;
                self.last_target_pos = Some(target.pos);
                return true;
            }
        }
        tick.erase_memory(MemoryModuleType::WalkTarget);
        if reached {
            tick.erase_memory(MemoryModuleType::CantReachWalkTargetSince);
        }
        false
    }

    // `MoveToTargetSink.canStillUse` (`:67-76`).
    fn can_still_use(
        &self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        if self.path.is_none() || self.last_target_pos.is_none() {
            return false;
        }
        let Some(target) = walk_target(tick) else {
            return false;
        };
        !tick.world().body_navigation_is_done()
            && !reached_target(tick, target)
            && !walk_target_is_spectator(tick)
    }

    // `MoveToTargetSink.start` (`:91-94`) follows the duration draw.
    fn start(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) {
        if let Some(path) = self.path {
            tick.set_memory_value(MemoryModuleType::Path, MemoryValue::Path(path));
        }
        tick.world_mut()
            .body_start_prepared_path(self.speed_modifier);
    }

    // `MoveToTargetSink.tick` (`:96-111`).
    fn tick(&mut self, tick: &mut BrainTick<'_>, context: &mut BrainTickContext, timestamp: i64) {
        let new_path = tick.world().body_path();
        if self.path != new_path {
            self.path = new_path;
            match new_path {
                Some(path) => {
                    tick.set_memory_value(MemoryModuleType::Path, MemoryValue::Path(path))
                }
                None => tick.erase_memory(MemoryModuleType::Path),
            }
        }
        if new_path.is_none() {
            return;
        }
        let (Some(last_target_pos), Some(target)) = (self.last_target_pos, walk_target(tick))
        else {
            return;
        };
        // `walkTarget.getTarget().currentBlockPosition().distSqr(lastTargetPos) > 4.0`
        // (`:106`).
        if block_pos_dist_sqr(target.pos, last_target_pos) <= 4.0 {
            return;
        }
        self.path = try_compute_path(tick, context, target);
        if self.path.is_some() {
            self.speed_modifier = target.speed_modifier;
            self.last_target_pos = Some(target.pos);
            self.start(tick, context, timestamp);
        }
    }

    // `MoveToTargetSink.stop` (`:78-89`).
    fn stop(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, _timestamp: i64) {
        if let Some(target) = walk_target(tick) {
            if !reached_target(tick, target) && tick.world().body_navigation_is_stuck() {
                // `level.getRandom().nextInt(40)` (`:82`).
                self.remaining_cooldown = tick.world_mut().level_random_next_int(40);
            }
        }
        tick.world_mut().stop_body_navigation();
        tick.erase_memory(MemoryModuleType::WalkTarget);
        tick.erase_memory(MemoryModuleType::Path);
        self.last_target_pos = None;
        self.path = None;
    }

    fn debug_string(&self) -> &'static str {
        "MoveToTargetSink"
    }
}

/// `new MoveToTargetSink()`.
pub fn move_to_target_sink() -> Behavior {
    // The entry condition is a registered `CANT_REACH_WALK_TARGET_SINCE`, an
    // absent `PATH` and a present `WALK_TARGET`.
    Behavior::with_duration(
        vec![
            (
                MemoryModuleType::CantReachWalkTargetSince,
                MemoryStatus::Registered,
            ),
            (MemoryModuleType::Path, MemoryStatus::ValueAbsent),
            (MemoryModuleType::WalkTarget, MemoryStatus::ValuePresent),
        ],
        150,
        250,
        MoveToTargetSink::default(),
    )
}

/// `MoveToTargetSink.tryComputePath`.
fn try_compute_path(
    tick: &mut BrainTick<'_>,
    context: &mut BrainTickContext,
    target: WalkTargetValue,
) -> Option<super::memory::MemoryPath> {
    let reached = reached_target(tick, target);
    let path = tick.world_mut().body_prepare_path(target.pos, 0);
    if reached {
        tick.erase_memory(MemoryModuleType::CantReachWalkTargetSince);
    } else {
        if path.is_some_and(|path| path.reached) {
            tick.erase_memory(MemoryModuleType::CantReachWalkTargetSince);
        } else if !tick.check_memory(
            MemoryModuleType::CantReachWalkTargetSince,
            MemoryStatus::ValuePresent,
        ) {
            let timestamp = tick.world().game_time();
            tick.set_memory_value(
                MemoryModuleType::CantReachWalkTargetSince,
                MemoryValue::Long(timestamp),
            );
        }
    }
    if reached {
        return None;
    }
    if path.is_some() {
        return path;
    }
    let towards = Vec3d {
        x: f64::from(target.pos.x) + 0.5,
        y: f64::from(target.pos.y),
        z: f64::from(target.pos.z) + 0.5,
    };
    let partial = tick.world_mut().body_default_random_pos_towards(
        context.body_random(),
        10,
        7,
        towards,
        f64::from(std::f32::consts::FRAC_PI_2),
    )??;
    tick.world_mut()
        .body_prepare_path(containing_block_pos(partial), 0)
}

/// `MoveToTargetSink.reachedTarget`.
fn reached_target(tick: &BrainTick<'_>, target: WalkTargetValue) -> bool {
    let body = tick.world().body().block_position;
    (target.pos.x - body.x).abs() + (target.pos.y - body.y).abs() + (target.pos.z - body.z).abs()
        <= target.close_enough_dist
}

fn block_pos_dist_sqr(left: BlockPos, right: BlockPos) -> f64 {
    let dx = f64::from(left.x - right.x);
    let dy = f64::from(left.y - right.y);
    let dz = f64::from(left.z - right.z);
    dx * dx + dy * dy + dz * dz
}

/// `WalkTarget.Vec3`/`BlockPos` constructors, which
/// both become a `BlockPosTracker` over the containing block.
pub(super) fn make_walk_target(
    position: Vec3d,
    speed_modifier: f32,
    close_enough: i32,
) -> MemoryWalkTarget {
    MemoryWalkTarget::at(containing_block_pos(position), speed_modifier, close_enough)
}

/// `BlockPos.containing(Vec3)`, the floor of each component.
pub(crate) fn containing_block_pos(position: Vec3d) -> BlockPos {
    BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    }
}

/// `new EntityTracker(entity, _)` over an entity
/// the level can still place, whose position seeds the tracker's stand-in.
pub(crate) fn entity_tracker(
    tick: &BrainTick<'_>,
    entity_id: i32,
) -> Option<MemoryPositionTracker> {
    entity_position(tick, entity_id).map(|position| MemoryPositionTracker::Entity {
        entity_id,
        last_seen: containing_block_pos(position),
    })
}

/// `PositionTracker.currentBlockPosition()`, which
/// the `EntityTracker` arm answers from the tracked entity as of this call
fn tracker_block_position(tick: &BrainTick<'_>, tracker: MemoryPositionTracker) -> BlockPos {
    match tracker {
        MemoryPositionTracker::BlockPos(pos) => pos,
        MemoryPositionTracker::Entity {
            entity_id,
            last_seen,
        } => entity_position(tick, entity_id).map_or(last_seen, containing_block_pos),
    }
}

#[derive(Debug, Clone, Copy)]
struct WalkTargetValue {
    pos: BlockPos,
    speed_modifier: f32,
    close_enough_dist: i32,
}

/// The walk target as every reader of it sees it: `getTarget()` already
/// resolved through `currentBlockPosition()`, which is the read Vanilla does
/// at each of `MoveToTargetSink`'s four call sites (`MoveToTargetSink`).
fn walk_target(tick: &BrainTick<'_>) -> Option<WalkTargetValue> {
    let Some(MemoryValue::WalkTarget(target)) = tick.memory(MemoryModuleType::WalkTarget) else {
        return None;
    };
    let target = *target;
    Some(WalkTargetValue {
        pos: tracker_block_position(tick, target.target),
        speed_modifier: target.speed_modifier,
        close_enough_dist: target.close_enough_dist,
    })
}

/// `MoveToTargetSink.isWalkTargetSpectator`.
fn walk_target_is_spectator(tick: &BrainTick<'_>) -> bool {
    let Some(MemoryValue::WalkTarget(target)) = tick.memory(MemoryModuleType::WalkTarget) else {
        return false;
    };
    let MemoryPositionTracker::Entity { entity_id, .. } = target.target else {
        return false;
    };
    tick.world()
        .entity_facts(entity_id)
        .is_some_and(|facts| facts.spectator)
}

fn entity_position(tick: &BrainTick<'_>, entity_id: i32) -> Option<Vec3d> {
    tick.world()
        .entity_facts(entity_id)
        .map(|facts| facts.position)
}

fn entity_is_alive(tick: &BrainTick<'_>, entity_id: i32) -> bool {
    tick.world()
        .entity_facts(entity_id)
        .is_some_and(|facts| facts.alive)
}

/// `EntityTracker.isVisibleBy` over
/// `NearestVisibleLivingEntities.contains`.
fn body_sees_entity(tick: &BrainTick<'_>, entity_id: i32) -> bool {
    let visible = matches!(
        tick.memory(MemoryModuleType::NearestVisibleLivingEntities),
        Some(MemoryValue::VisibleEntities(ids)) if ids.contains(&entity_id)
    );
    visible && is_entity_targetable(tick, entity_id)
}

/// `ShootTongue`'s
/// `target.position().vectorTo(body.position()).normalize().scale(0.75)`,
/// whose `normalize` answers the zero vector below `1.0E-5`
fn pull_toward_body(tick: &BrainTick<'_>, target: i32) -> Option<Vec3d> {
    let from = entity_position(tick, target)?;
    let to = tick.world().body().position;
    let delta = Vec3d {
        x: to.x - from.x,
        y: to.y - from.y,
        z: to.z - from.z,
    };
    let length = (delta.x * delta.x + delta.y * delta.y + delta.z * delta.z).sqrt();
    if length < 1.0e-5 {
        return None;
    }
    let scale = ShootTongue::EATING_MOVEMENT_FACTOR / length;
    Some(Vec3d {
        x: delta.x * scale,
        y: delta.y * scale,
        z: delta.z * scale,
    })
}

/// `ShootTongue.canPathfindToTarget`: a path must
/// exist and end within `EATING_DISTANCE` of the target.
fn can_pathfind_to_target(tick: &BrainTick<'_>, target: i32) -> bool {
    tick.world()
        .body_path_distance_to(target, 0)
        .is_some_and(|distance| distance < ShootTongue::EATING_DISTANCE)
}

/// `ShootTongue.addUnreachableTargetToMemory`.
fn remember_unreachable_target(tick: &mut BrainTick<'_>, target: i32) {
    let Some(uuid) = tick.world().entity_facts(target).map(|facts| facts.uuid) else {
        return;
    };
    let mut unreachable = match tick.memory(MemoryModuleType::UnreachableTongueTargets) {
        Some(MemoryValue::UuidList(uuids)) => uuids.clone(),
        _ => Vec::new(),
    };
    let should_add = !unreachable.contains(&uuid);
    if unreachable.len() == ShootTongue::MAX_UNREACHABLE_TARGETS && should_add {
        unreachable.remove(0);
    }
    if should_add {
        unreachable.push(uuid);
    }
    tick.set_memory_with_expiry(
        MemoryModuleType::UnreachableTongueTargets,
        MemoryValue::UuidList(unreachable),
        ShootTongue::UNREACHABLE_COOLDOWN,
    );
}

/// `Mob.doHurtTarget`'s `attacker.damageSources().mobAttack(this)`
/// (`Mob.java:1358`) for a body whose main hand holds nothing.
pub const MOB_ATTACK_DAMAGE_TYPE: &str = "minecraft:mob_attack";

/// `UniformInt.sample` over `Mth.randomBetweenInclusive`
/// `minInclusive` for a range
/// that cannot span, otherwise `min + nextInt(max - min + 1)`.
///
/// Every caller passes `level.getRandom()` for that draw
fn uniform_int_span_from_level(
    tick: &mut BrainTick<'_>,
    min_inclusive: i32,
    max_inclusive: i32,
) -> i32 {
    if min_inclusive >= max_inclusive {
        return min_inclusive;
    }
    min_inclusive
        + tick
            .world_mut()
            .level_random_next_int(max_inclusive - min_inclusive + 1)
}

/// `Swim`, which `CreakingAi`'s CORE registers as
/// `new Swim<Creaking>(0.8F) { checkExtraStartConditions: body.canMove() &&
/// super.checkExtraStartConditions(...) }`.
#[derive(Debug, Clone, Copy)]
pub struct Swim {
    /// `Swim.chance`, the `nextFloat` comparison `tick` makes.
    chance: f32,
    /// The anonymous subclass's extra term.
    requires_can_move: bool,
}

impl Swim {
    /// `Swim.shouldSwim`.
    fn should_swim(tick: &BrainTick<'_>) -> bool {
        let world = tick.world();
        (world.body().in_water
            && world.body().water_fluid_height > world.body().fluid_jump_threshold())
            || world.body().in_lava
    }

    fn wants_to_run(&self, tick: &BrainTick<'_>) -> bool {
        (!self.requires_can_move || tick.world().body().can_move) && Self::should_swim(tick)
    }
}

impl BehaviorHooks for Swim {
    // `Swim.checkExtraStartConditions`.
    fn extra_start_conditions(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
    ) -> bool {
        self.wants_to_run(tick)
    }

    // `Swim.canStillUse` is the same condition.
    fn can_still_use(
        &self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        self.wants_to_run(tick)
    }

    // `Swim.tick`: the draw is the body's own random
    // (`body.getRandom()`), which is the stream this context carries.
    fn tick(&mut self, tick: &mut BrainTick<'_>, context: &mut BrainTickContext, _timestamp: i64) {
        if context.next_float() < self.chance {
            tick.world_mut().request_body_jump();
        }
    }

    fn debug_string(&self) -> &'static str {
        "Swim"
    }
}

/// `new Swim<Creaking>(0.8F)`.
///
/// `requires_can_move` is the override `CreakingAi`'s anonymous subclass adds;
/// Vanilla's own `Swim` registers every other term with it false.
pub fn swim(chance: f32, requires_can_move: bool) -> Behavior {
    // `super(ImmutableMap.of())` leaves the entry condition
    // empty and `Behavior`'s own sixty-tick duration in place.
    Behavior::new(
        Vec::new(),
        Swim {
            chance,
            requires_can_move,
        },
    )
}

/// `SetEntityLookTargetSometimes`,
/// which `CreakingAi`'s IDLE registers as
/// `create(8.0F, UniformInt.of(30, 60))`.
#[derive(Debug, Clone, Copy)]
pub struct SetEntityLookTargetSometimes {
    /// The `EntityType` the `create(EntityType, float, UniformInt)` overload
    /// filters the visible list with (`:19-21`), or nothing for the
    /// `create(float, UniformInt)` overload the creaking uses (`:15-17`).
    entity_resource_id: Option<&'static str>,
    /// `maxDistSqr` (`:24`).
    max_dist_sqr: f64,
    /// `Ticker.interval`'s bounds (`:49`).
    interval_min: i32,
    interval_max: i32,
    /// `Ticker.ticksUntilNextStart` (`:50`), which outlives the tick that drew
    /// it: the ticker is built once per behaviour (`:25`).
    ticks_until_next_start: i32,
}

impl SetEntityLookTargetSometimes {
    /// `Ticker.tickDownAndCheck` (`:60-67`): a zero counter arms itself from
    /// the interval and answers false, every other count answers true exactly
    /// on the tick it reaches zero.
    ///
    /// The interval is `UniformInt.sample`'s `Mth.randomBetweenInclusive`
    /// and `:37` hands it `level.getRandom()` -- the
    /// level's own stream, not the body's own random.
    fn tick_down_and_check(&mut self, tick: &mut BrainTick<'_>) -> bool {
        if self.ticks_until_next_start == 0 {
            let span = self.interval_max + 1 - self.interval_min;
            let sampled = if self.interval_min >= self.interval_max {
                self.interval_min
            } else {
                self.interval_min + tick.world_mut().level_random_next_int(span)
            };
            self.ticks_until_next_start = sampled - 1;
            false
        } else {
            self.ticks_until_next_start -= 1;
            self.ticks_until_next_start == 0
        }
    }
}

impl BehaviorHooks for SetEntityLookTargetSometimes {
    // `SetEntityLookTargetSometimes.create`'s lambda (`:30-43`).
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        if !self.tick_down_and_check(tick) {
            return false;
        }
        let filter = self
            .entity_resource_id
            .map_or(LookTargetFilter::Any, LookTargetFilter::Resource);
        let Some(target) = closest_visible_look_target(tick, filter, self.max_dist_sqr, false)
        else {
            return false;
        };
        tick.set_memory_value(
            MemoryModuleType::LookTarget,
            MemoryValue::EntityTracker(target),
        );
        true
    }

    fn debug_string(&self) -> &'static str {
        "SetEntityLookTargetSometimes"
    }
}

/// `SetEntityLookTargetSometimes.create(8.0F, UniformInt.of(30, 60))`
/// with an inclusive 30..=60 interval.
pub fn set_entity_look_target_sometimes(
    max_dist: f32,
    interval_min: i32,
    interval_max: i32,
) -> Behavior {
    set_entity_look_target_sometimes_of(None, max_dist, interval_min, interval_max)
}

/// `SetEntityLookTargetSometimes.create(entityType, maxDist, interval)`
/// which the frog's three
/// activities register for `EntityType.PLAYER` (`FrogAi`).
pub fn set_entity_look_target_sometimes_of(
    entity_resource_id: Option<&'static str>,
    max_dist: f32,
    interval_min: i32,
    interval_max: i32,
) -> Behavior {
    // The group asks for an absent `LOOK_TARGET` and a present
    // `NEAREST_VISIBLE_LIVING_ENTITIES` (`:27`).
    Behavior::one_shot(
        vec![
            (MemoryModuleType::LookTarget, MemoryStatus::ValueAbsent),
            (
                MemoryModuleType::NearestVisibleLivingEntities,
                MemoryStatus::ValuePresent,
            ),
        ],
        SetEntityLookTargetSometimes {
            entity_resource_id,
            max_dist_sqr: f64::from(max_dist) * f64::from(max_dist),
            interval_min,
            interval_max,
            ticks_until_next_start: 0,
        },
    )
}

#[derive(Debug, Clone, Copy)]
pub(super) enum LookTargetFilter {
    Any,
    Resource(&'static str),
    Category(bbb_protocol::entity_types::VanillaEntityMobCategory),
}

fn closest_visible_look_target(
    tick: &BrainTick<'_>,
    filter: LookTargetFilter,
    max_dist_sqr: f64,
    exclude_passengers: bool,
) -> Option<i32> {
    let Some(MemoryValue::VisibleEntities(ids)) =
        tick.memory(MemoryModuleType::NearestVisibleLivingEntities)
    else {
        return None;
    };
    ids.iter().copied().find(|id| {
        let Some(facts) = tick.world().entity_facts(*id) else {
            return false;
        };
        let accepted = match filter {
            LookTargetFilter::Any => true,
            LookTargetFilter::Resource(resource) => facts.entity_resource_id == resource,
            LookTargetFilter::Category(category) => {
                bbb_protocol::entity_types::vanilla_entity_type_for_resource_id(
                    facts.entity_resource_id,
                )
                .is_some_and(|entry| entry.mob_category == category)
            }
        };
        accepted
            && (!exclude_passengers || !tick.world().body_has_passenger(*id))
            && tick
                .world()
                .entity_distance_to_sqr(*id)
                .is_some_and(|distance| distance <= max_dist_sqr)
            && body_sees_entity(tick, *id)
    })
}

#[derive(Clone)]
struct SetEntityLookTarget {
    filter: LookTargetFilter,
    max_dist_sqr: f64,
}

impl BehaviorHooks for SetEntityLookTarget {
    // `SetEntityLookTarget.create`.
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        let Some(target) = closest_visible_look_target(tick, self.filter, self.max_dist_sqr, true)
        else {
            return false;
        };
        tick.set_memory_value(
            MemoryModuleType::LookTarget,
            MemoryValue::EntityTracker(target),
        );
        true
    }
    fn debug_string(&self) -> &'static str {
        "SetEntityLookTarget"
    }
}

fn set_entity_look_target(filter: LookTargetFilter, max_dist: f64) -> Behavior {
    Behavior::one_shot(
        vec![
            (MemoryModuleType::LookTarget, MemoryStatus::ValueAbsent),
            (
                MemoryModuleType::NearestVisibleLivingEntities,
                MemoryStatus::ValuePresent,
            ),
        ],
        SetEntityLookTarget {
            filter,
            max_dist_sqr: max_dist * max_dist,
        },
    )
}

/// `DoNothing`, which `CreakingAi`'s `RunOne`
/// registers as `new DoNothing(30, 60)`.
#[derive(Debug, Clone, Copy, Default)]
pub struct DoNothing;

impl BehaviorHooks for DoNothing {
    // `DoNothing.tryStart` (`:30-35`) has no entry condition and draws its
    // duration from `level.getRandom()`, which is the generic
    // `Behavior.tryStart` draw.

    // `DoNothing.tickOrStop` (`:37-42`) only ends on the timestamp, so a
    // running `DoNothing` is usable until the duration above runs out and its
    // own `tick` body is empty.
    fn can_still_use(
        &self,
        _tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        true
    }

    fn debug_string(&self) -> &'static str {
        "DoNothing"
    }
}

/// `new DoNothing(minDuration, maxDuration)`.
pub fn do_nothing(min_duration: i32, max_duration: i32) -> Behavior {
    Behavior::with_duration(Vec::new(), min_duration, max_duration, DoNothing)
}

/// `RandomStroll.stroll(0.3F)`, which
/// `CreakingAi`'s `RunOne` registers with weight two.
#[derive(Debug, Clone, Copy)]
pub struct RandomStroll {
    speed_modifier: f32,
    swim: bool,
}

impl BehaviorHooks for RandomStroll {
    // `RandomStroll.strollFlyOrSwim`'s trigger.
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        if self.swim && !tick.world().body().in_water {
            return false;
        }
        let sampled = if self.swim {
            tick.world_mut().body_swim_target_pos(context.body_random())
        } else {
            tick.world_mut()
                .body_land_random_pos(context.body_random(), 10, 7)
        };
        let Some(sampled) = sampled else { return false };
        // `walkTarget.setOrErase(pathGoalPos.map(pos -> new WalkTarget(pos,
        // speedModifier, 0)))` (`:52`): a null position still answers true and
        // leaves the memory erased.
        match sampled {
            Some(position) => tick.set_memory_value(
                MemoryModuleType::WalkTarget,
                MemoryValue::WalkTarget(make_walk_target(position, self.speed_modifier, 0)),
            ),
            None => tick.erase_memory(MemoryModuleType::WalkTarget),
        }
        true
    }

    fn debug_string(&self) -> &'static str {
        "RandomStroll"
    }
}

/// `RandomStroll.stroll(0.3F)` (`CreakingAi`,
/// `RandomStroll`): `LandRandomPos.getPos(body, 10, 7)` with the
/// `mayStrollFromWater` predicate `true`.
pub fn random_stroll(speed_modifier: f32) -> Behavior {
    // The group asks for an absent `WALK_TARGET` (`:46`).
    Behavior::one_shot(
        vec![(MemoryModuleType::WalkTarget, MemoryStatus::ValueAbsent)],
        RandomStroll {
            speed_modifier,
            swim: false,
        },
    )
}

/// `RandomStroll.swim`.
pub fn random_stroll_swim(speed_modifier: f32) -> Behavior {
    Behavior::one_shot(
        vec![(MemoryModuleType::WalkTarget, MemoryStatus::ValueAbsent)],
        RandomStroll {
            speed_modifier,
            swim: true,
        },
    )
}

/// `SetWalkTargetFromLookTarget.create(0.3F, 3)` (`CreakingAi`).
#[derive(Debug, Clone, Copy)]
pub struct SetWalkTargetFromLookTarget {
    speed_modifier: f32,
    close_enough_distance: i32,
}

impl BehaviorHooks for SetWalkTargetFromLookTarget {
    // `SetWalkTargetFromLookTarget.create`'s lambda (`:20-27`).
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        // `walkTarget.set(new WalkTarget(i.get(lookTarget),
        // speedModifier.apply(body), closeEnoughDistance))` (`:25`): the walk
        // target takes the very tracker the look target holds, so an
        // `EntityTracker` look target keeps tracking through this behaviour.
        let Some(tracker) = look_target_tracker(tick) else {
            return false;
        };
        tick.set_memory_value(
            MemoryModuleType::WalkTarget,
            MemoryValue::WalkTarget(MemoryWalkTarget {
                target: tracker,
                speed_modifier: self.speed_modifier,
                close_enough_dist: self.close_enough_distance,
            }),
        );
        true
    }

    fn debug_string(&self) -> &'static str {
        "SetWalkTargetFromLookTarget"
    }
}

/// `SetWalkTargetFromLookTarget.create(0.3F, 3)`.
pub fn set_walk_target_from_look_target(
    speed_modifier: f32,
    close_enough_distance: i32,
) -> Behavior {
    // The group asks for an absent `WALK_TARGET` and a present `LOOK_TARGET`
    // (`:19`).
    Behavior::one_shot(
        vec![
            (MemoryModuleType::WalkTarget, MemoryStatus::ValueAbsent),
            (MemoryModuleType::LookTarget, MemoryStatus::ValuePresent),
        ],
        SetWalkTargetFromLookTarget {
            speed_modifier,
            close_enough_distance,
        },
    )
}

/// `RunOne` over `GateBehavior`
/// which `CreakingAi`'s IDLE registers with the
/// three weighted children.
///
/// The children are behaviours of this crate, so the gate owns their
/// lifecycle and ends inside the tick its last child ends
#[derive(Clone)]
pub struct RunOne {
    children: Vec<(Behavior, i32)>,
    ordered_try_all: bool,
    /// The order `ShufflingList.shuffle` produced,
    /// as indices into `children`.
    order: Vec<usize>,
}

impl RunOne {
    /// `ShufflingList.shuffle`: every entry draws
    /// one float and the list is sorted by `-pow(random, 1 / weight)`
    /// ascending, so a heavier entry sorts earlier more often.
    fn shuffle(&mut self, random: &GameRandom) {
        let mut weighted: Vec<(f64, usize)> = self
            .children
            .iter()
            .enumerate()
            .map(|(index, (_, weight))| {
                let exponent = f64::from(1.0_f32 / *weight as f32);
                (-f64::from(random.next_float()).powf(exponent), index)
            })
            .collect();
        // `List.sort` is stable, so equal keys keep the declared order.
        weighted.sort_by(|left, right| left.0.total_cmp(&right.0));
        self.order = weighted.into_iter().map(|(_, index)| index).collect();
    }
}

impl BehaviorHooks for RunOne {
    fn for_each_required_memory(&self, visit: &mut dyn FnMut(MemoryModuleType)) {
        for (child, _) in &self.children {
            child.for_each_required_memory(visit);
        }
    }

    // `GateBehavior.tryStart` has already marked
    // the gate running before shuffling and trying the children.
    fn start(&mut self, tick: &mut BrainTick<'_>, context: &mut BrainTickContext, timestamp: i64) {
        if !self.ordered_try_all {
            self.shuffle(&context.random);
        }
        for order_index in 0..self.children.len() {
            let index = if self.ordered_try_all {
                order_index
            } else {
                self.order[order_index]
            };
            let (child, _) = &mut self.children[index];
            if child.status() != BehaviorStatus::Stopped {
                continue;
            }
            if child.try_start(tick, context, timestamp) && !self.ordered_try_all {
                break;
            }
        }
    }

    // `GateBehavior.tickOrStop` (`:80-85`) ticks every running child; the
    // generic driver ends the gate through `stops_after_tick` below.
    fn tick(&mut self, tick: &mut BrainTick<'_>, context: &mut BrainTickContext, timestamp: i64) {
        for (child, _) in &mut self.children {
            if child.status() == BehaviorStatus::Running {
                child.tick_or_stop(tick, context, timestamp);
            }
        }
    }

    fn stop(&mut self, tick: &mut BrainTick<'_>, context: &mut BrainTickContext, timestamp: i64) {
        for (child, _) in &mut self.children {
            if child.status() == BehaviorStatus::Running {
                child.do_stop(tick, context, timestamp);
            }
        }
    }

    fn stops_after_tick(&self) -> bool {
        !self
            .children
            .iter()
            .any(|(child, _)| child.status() == BehaviorStatus::Running)
    }

    fn debug_string(&self) -> &'static str {
        // Vanilla's `GateBehavior.debugString` also names the running children
        // this crate's hook answers a `&'static
        // str`, so only the gate's own name is published.
        if self.ordered_try_all {
            "GateBehavior"
        } else {
            "RunOne"
        }
    }
}

/// `new RunOne<>(entryCondition, weightedBehaviors)`:
/// `RunOne(ImmutableMap.of(), ...)` for the creaking
/// and an absent `WALK_TARGET` for the frog's two packages
pub fn run_one(
    entry_condition: Vec<(MemoryModuleType, MemoryStatus)>,
    weighted: impl IntoIterator<Item = (i32, Behavior)>,
) -> Behavior {
    Behavior::gate(
        entry_condition,
        RunOne {
            children: weighted
                .into_iter()
                .map(|(weight, behavior)| (behavior, weight))
                .collect(),
            ordered_try_all: false,
            order: Vec::new(),
        },
    )
}

/// Frog SWIM's ordered, try-all gate.
pub fn ordered_try_all(
    entry_condition: Vec<(MemoryModuleType, MemoryStatus)>,
    weighted: impl IntoIterator<Item = (i32, Behavior)>,
) -> Behavior {
    Behavior::gate(
        entry_condition,
        RunOne {
            children: weighted
                .into_iter()
                .map(|(weight, behavior)| (behavior, weight))
                .collect(),
            ordered_try_all: true,
            order: Vec::new(),
        },
    )
}

/// `MeleeAttack`, which `CreakingAi`'s FIGHT
/// registers as `MeleeAttack.create(Creaking::canMove, 40)`
#[derive(Debug, Clone, Copy)]
pub struct MeleeAttack {
    /// `cooldownBetweenAttacks` (`:16`), the expiry `ATTACK_COOLING_DOWN`
    /// carries.
    cooldown_between_attacks: i32,
    /// The `canAttackPredicate` (`:28`), which the creaking fills with
    /// `Creaking::canMove`.
    requires_can_move: bool,
}

impl BehaviorHooks for MeleeAttack {
    // `MeleeAttack.create`'s lambda (`:26-40`).
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        let Some(target) = tick.memory_entity(MemoryModuleType::AttackTarget) else {
            return false;
        };
        if self.requires_can_move && !tick.world().body().can_move {
            return false;
        }
        // `!isHoldingUsableNonMeleeWeapon(body)` (`:29`, `:45-47`): a body
        // holding no item fails `Mob.canUseNonMeleeWeapon`'s default
        // so this term is false for every body whose main
        // hand is empty.
        if !is_within_attack_range(tick, target) {
            return false;
        }
        // `i.get(nearestEntities).contains(target)` (`:31`).
        if !body_sees_entity(tick, target) {
            return false;
        }
        // `lookTarget.set(new EntityTracker(target, true))` (`:32`).
        tick.set_memory_value(
            MemoryModuleType::LookTarget,
            MemoryValue::EntityTracker(target),
        );
        // `body.swing(InteractionHand.MAIN_HAND)` (`:33`) and
        // `body.doHurtTarget(level, target)` (`:34`), in that order, before the
        // cooldown the trigger sets last (`:35`).
        tick.world_mut()
            .queue_effect(BrainWorldEffect::SwingMainHand);
        tick.world_mut()
            .queue_effect(BrainWorldEffect::MeleeAttack { target_id: target });
        tick.set_memory_with_expiry(
            MemoryModuleType::AttackCoolingDown,
            MemoryValue::Bool(true),
            i64::from(self.cooldown_between_attacks),
        );
        true
    }

    fn debug_string(&self) -> &'static str {
        "MeleeAttack"
    }
}

/// `MeleeAttack.create(Creaking::canMove, 40)`.
pub fn melee_attack(cooldown_between_attacks: i32, requires_can_move: bool) -> Behavior {
    // The group asks for a registered `LOOK_TARGET`, a present
    // `ATTACK_TARGET`, an absent `ATTACK_COOLING_DOWN` and a present
    // `NEAREST_VISIBLE_LIVING_ENTITIES` (`:19-22`).
    Behavior::one_shot(
        vec![
            (MemoryModuleType::LookTarget, MemoryStatus::Registered),
            (MemoryModuleType::AttackTarget, MemoryStatus::ValuePresent),
            (
                MemoryModuleType::AttackCoolingDown,
                MemoryStatus::ValueAbsent,
            ),
            (
                MemoryModuleType::NearestVisibleLivingEntities,
                MemoryStatus::ValuePresent,
            ),
        ],
        MeleeAttack {
            cooldown_between_attacks,
            requires_can_move,
        },
    )
}

/// `FollowTemptation.TEMPTATION_COOLDOWN` (`FollowTemptation.java:19`).
const FOLLOW_TEMPTATION_COOLDOWN: i32 = 100;

/// `FollowTemptation.DEFAULT_CLOSE_ENOUGH_DIST`
/// (`FollowTemptation.java:20`), which the one-argument constructor the frog
/// uses fills `closeEnoughDistance` with (`:26-28`).
const DEFAULT_CLOSE_ENOUGH_DIST: f64 = 2.5;

/// `FollowTemptation`, which `FrogAi`
/// registers in IDLE and in SWIM as `new FollowTemptation(s -> 1.25F)`
#[derive(Debug, Clone, Copy)]
pub struct FollowTemptation {
    /// `speedModifier.apply(body)` (`:53-55`).
    speed_modifier: f32,
    /// `closeEnoughDistance.apply(body)` (`:88`).
    close_enough_distance: f64,
}

impl BehaviorHooks for FollowTemptation {
    // `FollowTemptation.timedOut` (`:61-64`) refuses the duration the generic
    // behaviour drew.
    fn timed_out(&self, _timestamp: i64, _end_timestamp: i64) -> bool {
        false
    }

    // `FollowTemptation.canStillUse` (`:66-70`).
    fn can_still_use(
        &self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        tick.has_memory_value(MemoryModuleType::TemptingPlayer)
            && !tick.has_memory_value(MemoryModuleType::BreedTarget)
            && !tick.has_memory_value(MemoryModuleType::IsPanicking)
    }

    // `FollowTemptation.start` (`:72-74`).
    fn start(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) {
        tick.set_memory_value(MemoryModuleType::IsTempted, MemoryValue::Bool(true));
    }

    // `FollowTemptation.stop` (`:76-82`): the cooldown is what CORE's
    // `CountDownCooldownTicks` counts back down, so the two behaviours are the
    // two halves of the same gate.
    fn stop(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, _timestamp: i64) {
        tick.set_memory_value(
            MemoryModuleType::TemptationCooldownTicks,
            MemoryValue::Int(FOLLOW_TEMPTATION_COOLDOWN),
        );
        tick.erase_memory(MemoryModuleType::IsTempted);
        tick.erase_memory(MemoryModuleType::WalkTarget);
        tick.erase_memory(MemoryModuleType::LookTarget);
    }

    // `FollowTemptation.tick` (`:84-96`).
    fn tick(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, _timestamp: i64) {
        // `getTemptingPlayer(body).get()` (`:85`), which `canStillUse` has
        // already answered present for.
        let Some(player) = tick.memory_entity(MemoryModuleType::TemptingPlayer) else {
            return;
        };
        // `lookTarget.set(new EntityTracker(player, true))` (`:87`), written
        // before the walk target is judged.
        tick.set_memory_value(
            MemoryModuleType::LookTarget,
            MemoryValue::EntityTracker(player),
        );
        let close_enough = self.close_enough_distance * self.close_enough_distance;
        if tick
            .world()
            .entity_distance_to_sqr(player)
            .is_some_and(|distance| distance < close_enough)
        {
            // `walkTarget.erase()` (`:90`).
            tick.erase_memory(MemoryModuleType::WalkTarget);
            return;
        }
        // `new WalkTarget(new EntityTracker(player, lookInTheEyes,
        // lookInTheEyes), speedModifier, 2)` (`:92-94`). The frog's tracker
        // carries `lookInTheEyes` false, so its block position is the player's
        // own.
        if let Some(tracker) = entity_tracker(tick, player) {
            tick.set_memory_value(
                MemoryModuleType::WalkTarget,
                MemoryValue::WalkTarget(MemoryWalkTarget {
                    target: tracker,
                    speed_modifier: self.speed_modifier,
                    close_enough_dist: 2,
                }),
            );
        }
    }

    fn debug_string(&self) -> &'static str {
        "FollowTemptation"
    }
}

/// `new FollowTemptation(s -> 1.25F)`.
pub fn follow_temptation(speed_modifier: f32) -> Behavior {
    // `super(ImmutableMap.builder()...)` (`:37-47`).
    Behavior::new(
        vec![
            (MemoryModuleType::LookTarget, MemoryStatus::Registered),
            (MemoryModuleType::WalkTarget, MemoryStatus::Registered),
            (
                MemoryModuleType::TemptationCooldownTicks,
                MemoryStatus::ValueAbsent,
            ),
            (MemoryModuleType::IsTempted, MemoryStatus::ValueAbsent),
            (MemoryModuleType::TemptingPlayer, MemoryStatus::ValuePresent),
            (MemoryModuleType::BreedTarget, MemoryStatus::ValueAbsent),
            (MemoryModuleType::IsPanicking, MemoryStatus::ValueAbsent),
        ],
        FollowTemptation {
            speed_modifier,
            close_enough_distance: DEFAULT_CLOSE_ENOUGH_DIST,
        },
    )
}

#[cfg(test)]
mod tests;
