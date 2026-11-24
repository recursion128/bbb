//! The villager job-site behaviours of `VillagerGoalPackages`' CORE and WORK
//! packages: finding, claiming and keeping
//! a workstation, the profession it gives, and working at it.

use bbb_protocol::packets::{BlockPos, Vec3d};

use super::super::effect::BrainWorldEffect;
use super::super::memory::{
    MemoryGlobalPos, MemoryModuleType, MemoryPositionTracker, MemoryStatus, MemoryValue,
    MemoryWalkTarget,
};
use super::super::{Behavior, BehaviorHooks, BrainTick, BrainTickContext};
use crate::entity::ai::mob::brain_runtime::VillagerBrainFacts;
use crate::entity::ai::mob::VILLAGER_REQUIRED_PATH_LENGTH as REQUIRED_PATH_LENGTH;

/// `AcquirePoi.SCAN_RANGE` (`AcquirePoi.java:28`).
const SCAN_RANGE: i32 = 48;
/// `AcquirePoi`'s `.limit(5L)` (`AcquirePoi.java:99`).
const SCAN_LIMIT: usize = 5;
const NONE: &str = "minecraft:none";
const NITWIT: &str = "minecraft:nitwit";

fn villager(tick: &BrainTick<'_>) -> Option<VillagerBrainFacts> {
    tick.world().body().villager
}

/// `VillagerProfession.heldJobSite`: a
/// trading profession holds the POI type of its own name.
fn holds(profession: &str, poi_type: &str) -> bool {
    !matches!(profession, NONE | NITWIT) && profession == poi_type
}

/// `VillagerProfession.acquirableJobSite`: `NONE` takes any
/// `#acquirable_job_site`, `NITWIT` nothing.
fn acquires(tick: &BrainTick<'_>, profession: &str, poi_type: &str) -> bool {
    match profession {
        NONE => tick.world().poi_is_acquirable_job_site(poi_type),
        NITWIT => false,
        _ => profession == poi_type,
    }
}

pub(super) fn global_pos(
    tick: &BrainTick<'_>,
    memory: MemoryModuleType,
) -> Option<MemoryGlobalPos> {
    match tick.memory(memory)? {
        MemoryValue::GlobalPos(pos) => Some(pos.clone()),
        _ => None,
    }
}

pub(super) fn in_this_level(tick: &BrainTick<'_>, pos: &MemoryGlobalPos) -> bool {
    pos.dimension == tick.world().dimension()
}

/// `Vec3i.closerToCenterThan`.
pub(super) fn closer_to_center_than(pos: BlockPos, position: Vec3d, distance: f64) -> bool {
    let dx = f64::from(pos.x) + 0.5 - position.x;
    let dy = f64::from(pos.y) + 0.5 - position.y;
    let dz = f64::from(pos.z) + 0.5 - position.z;
    dx * dx + dy * dy + dz * dz < distance * distance
}

fn walk_to(tick: &mut BrainTick<'_>, pos: BlockPos, speed_modifier: f32, close_enough: i32) {
    tick.set_memory_value(
        MemoryModuleType::WalkTarget,
        MemoryValue::WalkTarget(MemoryWalkTarget::at(pos, speed_modifier, close_enough)),
    );
}

/// `PoiTypes.HOME` and `PoiTypes.MEETING`, with the meeting point's valid
/// range (`PoiTypes.java:137-138`).
const HOME: &str = crate::entity::villager::HOME_POI;
const MEETING: &str = crate::entity::villager::MEETING_POI;
const MEETING_VALID_RANGE: i32 = 6;

fn memory_holds(tick: &BrainTick<'_>, memory: MemoryModuleType, poi_type: &str) -> bool {
    let profession = villager(tick).map_or(NONE, |villager| villager.profession);
    crate::entity::villager::memory_holds_poi(memory, profession, poi_type, |poi_type| {
        tick.world().poi_is_acquirable_job_site(poi_type)
    })
}

/// `Villager.releasePoi` for a site in this level.
pub(super) fn release_poi(tick: &mut BrainTick<'_>, memory: MemoryModuleType) {
    let Some(site) = global_pos(tick, memory) else {
        return;
    };
    if !in_this_level(tick, &site) {
        return;
    }
    if let Some(poi_type) = tick.world_mut().poi_type_at(site.pos) {
        if memory_holds(tick, memory, poi_type) {
            tick.world_mut().poi_release(site.pos);
        }
    }
}

/// `AcquirePoi.create` as the villager's CORE
/// package builds it: a job site into
/// `POTENTIAL_JOB_SITE`, a free bed into `HOME`, a bell into `MEETING_POINT`.
#[derive(Debug, Clone)]
pub struct AcquirePoi {
    memory: MemoryModuleType,
    next_scheduled_start: i64,
    unreachable: Vec<(BlockPos, i64)>,
}

impl BehaviorHooks for AcquirePoi {
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        timestamp: i64,
    ) -> bool {
        let Some(villager) = villager(tick) else {
            return false;
        };
        if villager.baby && self.memory != MemoryModuleType::Home {
            return false;
        }
        let game_time = tick.world().game_time();
        if self.next_scheduled_start == 0 {
            self.next_scheduled_start =
                game_time + i64::from(tick.world_mut().level_random_next_int(20));
            return false;
        }
        if game_time < self.next_scheduled_start {
            return false;
        }
        self.next_scheduled_start =
            timestamp + 20 + i64::from(tick.world_mut().level_random_next_int(20));
        let (poi_type, acquire_into, reach) = match self.memory {
            MemoryModuleType::Home => (Some(HOME), MemoryModuleType::Home, 1),
            MemoryModuleType::MeetingPoint => (
                Some(MEETING),
                MemoryModuleType::MeetingPoint,
                MEETING_VALID_RANGE,
            ),
            _ => match villager.profession {
                NONE => (None, MemoryModuleType::PotentialJobSite, 1),
                NITWIT => return true,
                profession => (Some(profession), MemoryModuleType::PotentialJobSite, 1),
            },
        };
        let center = tick.world().body().block_position;
        self.unreachable
            .retain(|(_, retry_at)| timestamp < *retry_at);
        let mut candidates = tick.world_mut().job_site_candidates(
            center,
            SCAN_RANGE,
            poi_type,
            SCAN_LIMIT,
            &|pos| !self.unreachable.iter().any(|(failed, _)| *failed == pos),
        );
        if self.memory == MemoryModuleType::Home {
            candidates.retain(|&pos| tick.world().bed_occupied(pos) == Some(false));
        }
        // AcquirePoi: one search for the whole candidate batch.
        match tick
            .world()
            .body_path_to_pois(&candidates, reach, REQUIRED_PATH_LENGTH)
        {
            Some((pos, true)) => {
                if tick.world_mut().poi_take(pos) {
                    self.unreachable.clear();
                    let dimension = tick.world().dimension().to_owned();
                    tick.set_memory_value(
                        acquire_into,
                        MemoryValue::GlobalPos(MemoryGlobalPos { dimension, pos }),
                    );
                    if self.memory != MemoryModuleType::JobSite {
                        tick.world_mut()
                            .queue_effect(BrainWorldEffect::BodyEntityEvent { event_id: 14 });
                    }
                }
            }
            Some((_, false)) => {
                for pos in candidates {
                    if self.unreachable.len() == 32 {
                        self.unreachable.remove(0);
                    }
                    self.unreachable.push((pos, timestamp + 400));
                }
            }
            None => {}
        }
        true
    }

    fn debug_string(&self) -> &'static str {
        "AcquirePoi"
    }
}

/// `memory` is `JOB_SITE` for the job-site claim, which fills
/// `POTENTIAL_JOB_SITE`, or the memory a home or meeting point fills.
pub fn acquire_poi(memory: MemoryModuleType) -> Behavior {
    let mut condition = vec![(memory, MemoryStatus::ValueAbsent)];
    if memory == MemoryModuleType::JobSite {
        condition.push((
            MemoryModuleType::PotentialJobSite,
            MemoryStatus::ValueAbsent,
        ));
    }
    Behavior::one_shot(
        condition,
        AcquirePoi {
            memory,
            next_scheduled_start: 0,
            unreachable: Vec::new(),
        },
    )
}

/// `GoToPotentialJobSite`: walk to the
/// claimed site, and give the claim back if the walk ends without a job.
#[derive(Debug, Clone, Copy)]
pub struct GoToPotentialJobSite {
    speed_modifier: f32,
}

impl BehaviorHooks for GoToPotentialJobSite {
    fn can_still_use(
        &self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        tick.has_memory_value(MemoryModuleType::PotentialJobSite)
    }

    fn tick(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, _timestamp: i64) {
        if let Some(pos) = global_pos(tick, MemoryModuleType::PotentialJobSite) {
            walk_to(tick, pos.pos, self.speed_modifier, 1);
        }
    }

    fn stop(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, _timestamp: i64) {
        if let Some(pos) = global_pos(tick, MemoryModuleType::PotentialJobSite) {
            if in_this_level(tick, &pos) {
                tick.world_mut().poi_release(pos.pos);
            }
        }
        tick.erase_memory(MemoryModuleType::PotentialJobSite);
    }

    fn debug_string(&self) -> &'static str {
        "GoToPotentialJobSite"
    }
}

/// `new GoToPotentialJobSite(speedModifier)`, whose duration is 1200 ticks
pub fn go_to_potential_job_site(speed_modifier: f32) -> Behavior {
    Behavior::with_duration(
        vec![(
            MemoryModuleType::PotentialJobSite,
            MemoryStatus::ValuePresent,
        )],
        1200,
        1200,
        GoToPotentialJobSite { speed_modifier },
    )
}

/// `AssignProfessionFromJobSite`.
#[derive(Debug, Clone, Copy)]
pub struct AssignProfessionFromJobSite;

impl BehaviorHooks for AssignProfessionFromJobSite {
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        let (Some(villager), Some(site)) = (
            villager(tick),
            global_pos(tick, MemoryModuleType::PotentialJobSite),
        ) else {
            return false;
        };
        let position = tick.world().body().position;
        if !closer_to_center_than(site.pos, position, 2.0)
            && !villager.assign_profession_when_spawned
        {
            return false;
        }
        tick.erase_memory(MemoryModuleType::PotentialJobSite);
        let pos = site.pos;
        let same_level = in_this_level(tick, &site);
        tick.set_memory_value(MemoryModuleType::JobSite, MemoryValue::GlobalPos(site));
        tick.world_mut()
            .queue_effect(BrainWorldEffect::BodyEntityEvent { event_id: 14 });
        if villager.profession != NONE || !same_level {
            return true;
        }
        if let Some(poi_type) = tick.world_mut().poi_type_at(pos) {
            if crate::entity::ai::mob::villager::PROFESSIONS
                .iter()
                .any(|profession| holds(profession, poi_type))
            {
                tick.world_mut()
                    .queue_effect(BrainWorldEffect::SetVillagerProfession {
                        profession: poi_type,
                    });
            }
        }
        true
    }

    fn debug_string(&self) -> &'static str {
        "AssignProfessionFromJobSite"
    }
}

pub fn assign_profession_from_job_site() -> Behavior {
    Behavior::one_shot(
        vec![
            (
                MemoryModuleType::PotentialJobSite,
                MemoryStatus::ValuePresent,
            ),
            (MemoryModuleType::JobSite, MemoryStatus::Registered),
        ],
        AssignProfessionFromJobSite,
    )
}

/// `ValidateNearbyPoi.create(predicate, memory)`:
/// a remembered POI that is gone, no longer fits, or is a bed someone else
/// sleeps in, is forgotten. A job site is checked against
/// `profession.heldJobSite()` / `acquirableJobSite()`
#[derive(Debug, Clone, Copy)]
pub struct ValidateNearbyPoi {
    memory: MemoryModuleType,
}

impl BehaviorHooks for ValidateNearbyPoi {
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        let (Some(villager), Some(site)) = (villager(tick), global_pos(tick, self.memory)) else {
            return false;
        };
        if !in_this_level(tick, &site)
            || !closer_to_center_than(site.pos, tick.world().body().position, 16.0)
        {
            return false;
        }
        let valid = tick
            .world_mut()
            .poi_type_at(site.pos)
            .is_some_and(|poi_type| match self.memory {
                MemoryModuleType::JobSite => holds(villager.profession, poi_type),
                MemoryModuleType::PotentialJobSite => acquires(tick, villager.profession, poi_type),
                memory => memory_holds(tick, memory, poi_type),
            });
        if !valid {
            tick.erase_memory(self.memory);
        } else if tick.world().bed_occupied(site.pos) == Some(true) && !villager.sleeping {
            // `bedIsOccupied` / `bedIsOccupiedByVillager`.
            tick.erase_memory(self.memory);
            if !tick.world().villager_sleeps_in(site.pos) {
                tick.world_mut().poi_release(site.pos);
            }
        }
        true
    }

    fn debug_string(&self) -> &'static str {
        "ValidateNearbyPoi"
    }
}

pub fn validate_nearby_poi(memory: MemoryModuleType) -> Behavior {
    Behavior::one_shot(
        vec![(memory, MemoryStatus::ValuePresent)],
        ValidateNearbyPoi { memory },
    )
}

/// `ResetProfession`: a villager that never
/// traded loses its profession with its job site.
#[derive(Debug, Clone, Copy)]
pub struct ResetProfession;

impl BehaviorHooks for ResetProfession {
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        let Some(villager) = villager(tick) else {
            return false;
        };
        if matches!(villager.profession, NONE | NITWIT) || villager.xp != 0 || villager.level > 1 {
            return false;
        }
        tick.world_mut()
            .queue_effect(BrainWorldEffect::SetVillagerProfession { profession: NONE });
        true
    }

    fn debug_string(&self) -> &'static str {
        "ResetProfession"
    }
}

pub fn reset_profession() -> Behavior {
    Behavior::one_shot(
        vec![(MemoryModuleType::JobSite, MemoryStatus::ValueAbsent)],
        ResetProfession,
    )
}

/// `WorkAtPoi`.
#[derive(Debug, Clone, Copy, Default)]
pub struct WorkAtPoi {
    last_check: i64,
}

/// `WorkAtPoi.DISTANCE` (`WorkAtPoi.java:14`).
const WORK_DISTANCE: f64 = 1.73;

fn at_job_site(tick: &BrainTick<'_>) -> bool {
    global_pos(tick, MemoryModuleType::JobSite).is_some_and(|site| {
        in_this_level(tick, &site)
            && closer_to_center_than(site.pos, tick.world().body().position, WORK_DISTANCE)
    })
}

impl BehaviorHooks for WorkAtPoi {
    fn extra_start_conditions(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
    ) -> bool {
        let game_time = tick.world().game_time();
        if game_time - self.last_check < 300 || tick.world_mut().level_random_next_int(2) != 0 {
            return false;
        }
        self.last_check = game_time;
        at_job_site(tick)
    }

    fn start(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, timestamp: i64) {
        tick.set_memory_value(
            MemoryModuleType::LastWorkedAtPoi,
            MemoryValue::Long(timestamp),
        );
        if let Some(site) = global_pos(tick, MemoryModuleType::JobSite) {
            tick.world_mut()
                .queue_effect(BrainWorldEffect::VillagerWorkedAtPoi { job_site: site.pos });
        }
    }

    fn can_still_use(
        &self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        at_job_site(tick)
    }

    fn debug_string(&self) -> &'static str {
        "WorkAtPoi"
    }
}

pub fn work_at_poi() -> Behavior {
    Behavior::new(
        vec![
            (MemoryModuleType::JobSite, MemoryStatus::ValuePresent),
            (MemoryModuleType::LookTarget, MemoryStatus::Registered),
        ],
        WorkAtPoi::default(),
    )
}

/// `StrollToPoi.create(memory, speedModifier, closeEnoughDist,
/// maxDistanceFromPoi)`.
#[derive(Debug, Clone, Copy)]
pub struct StrollToPoi {
    memory: MemoryModuleType,
    speed_modifier: f32,
    close_enough: i32,
    max_distance: i32,
    next_ok_start_time: i64,
}

impl BehaviorHooks for StrollToPoi {
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        timestamp: i64,
    ) -> bool {
        let Some(site) = global_pos(tick, self.memory) else {
            return false;
        };
        if !in_this_level(tick, &site)
            || !closer_to_center_than(
                site.pos,
                tick.world().body().position,
                f64::from(self.max_distance),
            )
        {
            return false;
        }
        if timestamp <= self.next_ok_start_time {
            return true;
        }
        walk_to(tick, site.pos, self.speed_modifier, self.close_enough);
        self.next_ok_start_time = timestamp + 80;
        true
    }

    fn debug_string(&self) -> &'static str {
        "StrollToPoi"
    }
}

pub fn stroll_to_poi(
    memory: MemoryModuleType,
    speed_modifier: f32,
    close_enough: i32,
    max_distance: i32,
) -> Behavior {
    Behavior::one_shot(
        vec![
            (MemoryModuleType::WalkTarget, MemoryStatus::Registered),
            (memory, MemoryStatus::ValuePresent),
        ],
        StrollToPoi {
            memory,
            speed_modifier,
            close_enough,
            max_distance,
            next_ok_start_time: 0,
        },
    )
}

/// `StrollToPoiList.create`: walk to one of
/// the listed sites while staying near another remembered one.
#[derive(Debug, Clone, Copy)]
pub struct StrollToPoiList {
    memory: MemoryModuleType,
    speed_modifier: f32,
    close_enough: i32,
    max_distance: i32,
    must_be_close_to: MemoryModuleType,
    next_ok_start_time: i64,
}

impl BehaviorHooks for StrollToPoiList {
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        timestamp: i64,
    ) -> bool {
        let site_count = match tick.memory(self.memory) {
            Some(MemoryValue::GlobalPosList(sites)) if !sites.is_empty() => sites.len(),
            Some(MemoryValue::SameDimensionPosList { positions, .. }) if !positions.is_empty() => {
                positions.len()
            }
            _ => return false,
        };
        let Some(stay_close_to) = global_pos(tick, self.must_be_close_to) else {
            return false;
        };
        let index = tick
            .world_mut()
            .level_random_next_int(i32::try_from(site_count).unwrap_or(i32::MAX));
        let index = usize::try_from(index).unwrap_or(0);
        let Some((same_level, site_pos)) = (match tick.memory(self.memory) {
            Some(MemoryValue::GlobalPosList(sites)) => sites
                .get(index)
                .map(|site| (in_this_level(tick, site), site.pos)),
            Some(MemoryValue::SameDimensionPosList {
                dimension,
                positions,
            }) => positions
                .get(index)
                .copied()
                .map(|pos| (dimension == tick.world().dimension(), pos)),
            _ => None,
        }) else {
            return false;
        };
        if !same_level
            || !closer_to_center_than(
                stay_close_to.pos,
                tick.world().body().position,
                f64::from(self.max_distance),
            )
        {
            return false;
        }
        if timestamp > self.next_ok_start_time {
            walk_to(tick, site_pos, self.speed_modifier, self.close_enough);
            self.next_ok_start_time = timestamp + 100;
        }
        true
    }

    fn debug_string(&self) -> &'static str {
        "StrollToPoiList"
    }
}

pub fn stroll_to_poi_list(
    memory: MemoryModuleType,
    speed_modifier: f32,
    close_enough: i32,
    max_distance: i32,
    must_be_close_to: MemoryModuleType,
) -> Behavior {
    Behavior::one_shot(
        vec![
            (MemoryModuleType::WalkTarget, MemoryStatus::Registered),
            (memory, MemoryStatus::ValuePresent),
            (must_be_close_to, MemoryStatus::ValuePresent),
        ],
        StrollToPoiList {
            memory,
            speed_modifier,
            close_enough,
            max_distance,
            must_be_close_to,
            next_ok_start_time: 0,
        },
    )
}

/// `SetWalkTargetFromBlockMemory.create(memory, ..)`
/// head back to the remembered
/// site, or give it up once it is unreachable for too long.
#[derive(Debug, Clone, Copy)]
pub struct SetWalkTargetFromBlockMemory {
    memory: MemoryModuleType,
    speed_modifier: f32,
    close_enough: i32,
    too_far: i32,
    too_long_unreachable: i64,
}

fn manhattan(a: BlockPos, b: BlockPos) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs() + (a.z - b.z).abs()
}

impl BehaviorHooks for SetWalkTargetFromBlockMemory {
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        context: &mut BrainTickContext,
        timestamp: i64,
    ) -> bool {
        let Some(site) = global_pos(tick, self.memory) else {
            return false;
        };
        let cant_reach_since = match tick.memory(MemoryModuleType::CantReachWalkTargetSince) {
            Some(MemoryValue::Long(since)) => Some(*since),
            _ => None,
        };
        let body = tick.world().body().block_position;
        let reachable = in_this_level(tick, &site)
            && cant_reach_since
                .is_none_or(|since| tick.world().game_time() - since <= self.too_long_unreachable);
        let distance = manhattan(site.pos, body);
        if reachable && distance > self.too_far {
            let towards = Vec3d {
                x: f64::from(site.pos.x) + 0.5,
                y: f64::from(site.pos.y),
                z: f64::from(site.pos.z) + 0.5,
            };
            let random = context.body_random().clone();
            for _ in 0..10 {
                let Some(Some(step)) = tick.world_mut().body_default_random_pos_towards(
                    &random,
                    15,
                    7,
                    towards,
                    std::f64::consts::FRAC_PI_2,
                ) else {
                    continue;
                };
                let step_pos = super::containing_block_pos(step);
                if manhattan(step_pos, body) <= self.too_far {
                    walk_to(tick, step_pos, self.speed_modifier, self.close_enough);
                    return true;
                }
            }
        } else if reachable {
            if distance > self.close_enough {
                walk_to(tick, site.pos, self.speed_modifier, self.close_enough);
            }
            return true;
        }
        release_poi(tick, self.memory);
        tick.erase_memory(self.memory);
        tick.set_memory_value(
            MemoryModuleType::CantReachWalkTargetSince,
            MemoryValue::Long(timestamp),
        );
        true
    }

    fn debug_string(&self) -> &'static str {
        "SetWalkTargetFromBlockMemory"
    }
}

pub fn set_walk_target_from_block_memory(
    memory: MemoryModuleType,
    speed_modifier: f32,
    close_enough: i32,
    too_far: i32,
    too_long_unreachable: i64,
) -> Behavior {
    Behavior::one_shot(
        vec![
            (
                MemoryModuleType::CantReachWalkTargetSince,
                MemoryStatus::Registered,
            ),
            (MemoryModuleType::WalkTarget, MemoryStatus::ValueAbsent),
            (memory, MemoryStatus::ValuePresent),
        ],
        SetWalkTargetFromBlockMemory {
            memory,
            speed_modifier,
            close_enough,
            too_far,
            too_long_unreachable,
        },
    )
}

/// `LookAndFollowTradingPlayerSink`.
#[derive(Debug, Clone, Copy)]
pub struct LookAndFollowTradingPlayer {
    speed_modifier: f32,
}

impl LookAndFollowTradingPlayer {
    fn trading_player(tick: &BrainTick<'_>) -> Option<i32> {
        let player = villager(tick)?.trading_player?;
        let body = tick.world().body();
        (!body.in_water && tick.world().entity_distance_to_sqr(player)? <= 16.0).then_some(player)
    }

    fn follow(&self, tick: &mut BrainTick<'_>) {
        let Some(player) = Self::trading_player(tick) else {
            return;
        };
        if let Some(position) = tick
            .world()
            .entity_facts(player)
            .map(|facts| facts.position)
        {
            tick.set_memory_value(
                MemoryModuleType::WalkTarget,
                MemoryValue::WalkTarget(MemoryWalkTarget {
                    target: MemoryPositionTracker::Entity {
                        entity_id: player,
                        last_seen: super::containing_block_pos(position),
                    },
                    speed_modifier: self.speed_modifier,
                    close_enough_dist: 2,
                }),
            );
        }
        tick.set_memory_value(
            MemoryModuleType::LookTarget,
            MemoryValue::EntityTracker(player),
        );
    }
}

impl BehaviorHooks for LookAndFollowTradingPlayer {
    fn extra_start_conditions(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
    ) -> bool {
        Self::trading_player(tick).is_some()
    }

    fn can_still_use(
        &self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        Self::trading_player(tick).is_some()
    }

    fn start(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) {
        self.follow(tick);
    }

    fn tick(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, _timestamp: i64) {
        self.follow(tick);
    }

    fn stop(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, _timestamp: i64) {
        tick.erase_memory(MemoryModuleType::WalkTarget);
        tick.erase_memory(MemoryModuleType::LookTarget);
    }

    fn timed_out(&self, _timestamp: i64, _end_timestamp: i64) -> bool {
        false
    }

    fn debug_string(&self) -> &'static str {
        "LookAndFollowTradingPlayerSink"
    }
}

pub fn look_and_follow_trading_player(speed_modifier: f32) -> Behavior {
    Behavior::new(
        vec![
            (MemoryModuleType::WalkTarget, MemoryStatus::Registered),
            (MemoryModuleType::LookTarget, MemoryStatus::Registered),
        ],
        LookAndFollowTradingPlayer { speed_modifier },
    )
}

fn nearby_villagers(tick: &BrainTick<'_>) -> Vec<(i32, crate::entity::ai::VillagerJobFacts)> {
    let body_id = tick.world().body().entity_id;
    let Some(MemoryValue::EntityList(ids)) = tick.memory(MemoryModuleType::NearestLivingEntities)
    else {
        return Vec::new();
    };
    ids.iter()
        .filter(|id| **id != body_id)
        .filter_map(|id| Some((*id, tick.world().villager_job_facts(*id)?)))
        .collect()
}

fn erase_other(tick: &mut BrainTick<'_>, entity_id: i32, memory_type: MemoryModuleType) {
    tick.world_mut()
        .queue_effect(BrainWorldEffect::SetMobBrainMemory {
            entity_id,
            memory_type,
            value: None,
        });
}

/// `PoiCompetitorScan`: of the villagers
/// holding the same job site, the most experienced keeps it.
#[derive(Debug, Clone, Copy)]
pub struct PoiCompetitorScan;

impl BehaviorHooks for PoiCompetitorScan {
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        let (Some(site), Some(body)) =
            (global_pos(tick, MemoryModuleType::JobSite), villager(tick))
        else {
            return false;
        };
        let Some(poi_type) = tick.world_mut().poi_type_at(site.pos) else {
            return true;
        };
        let body_id = tick.world().body().entity_id;
        let mut winner = (body_id, body.xp);
        for (id, facts) in nearby_villagers(tick) {
            if facts.job_site.as_ref() != Some(&site) || !holds(facts.profession, poi_type) {
                continue;
            }
            // `selectWinner`: a tie goes to the newcomer.
            let loser = if winner.1 > facts.xp {
                id
            } else {
                std::mem::replace(&mut winner, (id, facts.xp)).0
            };
            if loser == body_id {
                tick.erase_memory(MemoryModuleType::JobSite);
            } else {
                erase_other(tick, loser, MemoryModuleType::JobSite);
            }
        }
        true
    }

    fn debug_string(&self) -> &'static str {
        "PoiCompetitorScan"
    }
}

pub fn poi_competitor_scan() -> Behavior {
    Behavior::one_shot(
        vec![
            (MemoryModuleType::JobSite, MemoryStatus::ValuePresent),
            (
                MemoryModuleType::NearestLivingEntities,
                MemoryStatus::ValuePresent,
            ),
        ],
        PoiCompetitorScan,
    )
}

/// `YieldJobSite`: an unemployed villager hands a
/// workstation it is heading for to a nearby villager of that profession.
#[derive(Debug, Clone, Copy)]
pub struct YieldJobSite {
    speed_modifier: f32,
}

impl BehaviorHooks for YieldJobSite {
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        let Some(body) = villager(tick) else {
            return false;
        };
        if body.baby || body.profession != NONE {
            return false;
        }
        let Some(site) = global_pos(tick, MemoryModuleType::PotentialJobSite) else {
            return false;
        };
        let Some(poi_type) = tick.world_mut().poi_type_at(site.pos) else {
            return true;
        };
        let Some(poi) = bbb_facts::poi_types::vanilla_poi_type_for_resource_id(poi_type) else {
            return true;
        };
        let taker = nearby_villagers(tick).into_iter().find(|(id, facts)| {
            !facts.has_potential_job_site
                && holds(facts.profession, poi_type)
                && match &facts.job_site {
                    Some(job_site) => job_site.pos == site.pos,
                    None => {
                        tick.world()
                            .villager_can_reach_job_site(*id, site.pos, poi.valid_range)
                    }
                }
        });
        if let Some((taker, facts)) = taker {
            tick.erase_memory(MemoryModuleType::WalkTarget);
            tick.erase_memory(MemoryModuleType::LookTarget);
            tick.erase_memory(MemoryModuleType::PotentialJobSite);
            if facts.job_site.is_none() {
                let set = |memory_type, value| BrainWorldEffect::SetMobBrainMemory {
                    entity_id: taker,
                    memory_type,
                    value: Some(value),
                };
                tick.world_mut().queue_effect(set(
                    MemoryModuleType::WalkTarget,
                    MemoryValue::WalkTarget(MemoryWalkTarget::at(site.pos, self.speed_modifier, 1)),
                ));
                tick.world_mut().queue_effect(set(
                    MemoryModuleType::LookTarget,
                    MemoryValue::BlockPos(site.pos),
                ));
                tick.world_mut().queue_effect(set(
                    MemoryModuleType::PotentialJobSite,
                    MemoryValue::GlobalPos(site),
                ));
            }
        }
        true
    }

    fn debug_string(&self) -> &'static str {
        "YieldJobSite"
    }
}

pub fn yield_job_site(speed_modifier: f32) -> Behavior {
    Behavior::one_shot(
        vec![
            (
                MemoryModuleType::PotentialJobSite,
                MemoryStatus::ValuePresent,
            ),
            (MemoryModuleType::JobSite, MemoryStatus::ValueAbsent),
            (
                MemoryModuleType::NearestLivingEntities,
                MemoryStatus::ValuePresent,
            ),
            (MemoryModuleType::WalkTarget, MemoryStatus::Registered),
            (MemoryModuleType::LookTarget, MemoryStatus::Registered),
        ],
        YieldJobSite { speed_modifier },
    )
}
