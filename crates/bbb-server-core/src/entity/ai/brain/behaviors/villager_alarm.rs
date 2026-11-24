//! What a villager does when it is hurt, sees a hostile or hears a bell:
//! the PANIC and HIDE packages (`VillagerGoalPackages`)
//! and the golem call of `VillagerPanicTrigger`.

use bbb_protocol::packets::{BlockPos, Vec3d};

use super::super::effect::BrainWorldEffect;
use super::super::memory::{
    MemoryGlobalPos, MemoryModuleType, MemoryStatus, MemoryValue, MemoryWalkTarget,
};
use super::super::{Behavior, BehaviorHooks, BrainTick, BrainTickContext};
use super::villager_work::{closer_to_center_than, global_pos};
use super::{containing_block_pos, make_walk_target};

/// `VillagerPanicTrigger.isHurt || hasHostile`
fn scared(tick: &BrainTick<'_>) -> bool {
    tick.has_memory_value(MemoryModuleType::HurtBy)
        || tick.has_memory_value(MemoryModuleType::NearestHostile)
}

/// `VillagerPanicTrigger`. The switch
/// into PANIC belongs to the villager's activity update; this keeps the
/// golem call that runs while the villager stays scared.
#[derive(Debug, Clone, Copy)]
pub struct VillagerPanicTrigger;

impl BehaviorHooks for VillagerPanicTrigger {
    fn extra_start_conditions(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
    ) -> bool {
        scared(tick)
    }

    fn can_still_use(
        &self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        scared(tick)
    }

    fn tick(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, timestamp: i64) {
        if timestamp % 100 == 0 {
            tick.world_mut()
                .queue_effect(BrainWorldEffect::VillagerSpawnGolem { needed: 3 });
        }
    }

    fn debug_string(&self) -> &'static str {
        "VillagerPanicTrigger"
    }
}

pub fn villager_panic_trigger() -> Behavior {
    Behavior::new(Vec::new(), VillagerPanicTrigger)
}

/// `VillagerCalmDown`: a villager no longer
/// scared forgets who hurt it, which lets its activity update leave PANIC.
#[derive(Debug, Clone, Copy)]
pub struct VillagerCalmDown;

impl BehaviorHooks for VillagerCalmDown {
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        let attacker_close = match tick.memory(MemoryModuleType::HurtByEntity) {
            Some(MemoryValue::EntityRef(entity_id)) => tick
                .world()
                .entity_distance_to_sqr(*entity_id)
                .is_some_and(|distance| distance <= 36.0),
            _ => false,
        };
        if !scared(tick) && !attacker_close {
            tick.erase_memory(MemoryModuleType::HurtBy);
            tick.erase_memory(MemoryModuleType::HurtByEntity);
        }
        true
    }

    fn debug_string(&self) -> &'static str {
        "VillagerCalmDown"
    }
}

pub fn villager_calm_down() -> Behavior {
    Behavior::one_shot(
        vec![
            (MemoryModuleType::HurtBy, MemoryStatus::Registered),
            (MemoryModuleType::HurtByEntity, MemoryStatus::Registered),
            (MemoryModuleType::NearestHostile, MemoryStatus::Registered),
        ],
        VillagerCalmDown,
    )
}

/// `SetWalkTargetAwayFrom.entity(memory, speed, desiredDistance, false)`.
/// Without `interruptCurrentWalk` a
/// present walk target refuses the start, so the direction test never runs.
#[derive(Debug, Clone, Copy)]
pub struct SetWalkTargetAwayFrom {
    memory: MemoryModuleType,
    speed_modifier: f32,
    desired_distance: f64,
}

impl BehaviorHooks for SetWalkTargetAwayFrom {
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        if tick.has_memory_value(MemoryModuleType::WalkTarget) {
            return false;
        }
        let Some(MemoryValue::EntityRef(entity_id)) = tick.memory(self.memory) else {
            return false;
        };
        let Some(avoid) = tick
            .world()
            .entity_facts(*entity_id)
            .map(|facts| facts.position)
        else {
            return false;
        };
        let body = tick.world().body().position;
        let (dx, dy, dz) = (body.x - avoid.x, body.y - avoid.y, body.z - avoid.z);
        if dx * dx + dy * dy + dz * dz >= self.desired_distance * self.desired_distance {
            return false;
        }
        for _ in 0..10 {
            if let Some(Some(flee_to)) =
                tick.world_mut()
                    .body_land_random_pos_away(context.body_random(), 16, 7, avoid)
            {
                tick.set_memory_value(
                    MemoryModuleType::WalkTarget,
                    MemoryValue::WalkTarget(make_walk_target(flee_to, self.speed_modifier, 0)),
                );
                break;
            }
        }
        true
    }

    fn debug_string(&self) -> &'static str {
        "SetWalkTargetAwayFrom"
    }
}

pub fn set_walk_target_away_from(
    memory: MemoryModuleType,
    speed_modifier: f32,
    desired_distance: i32,
) -> Behavior {
    Behavior::one_shot(
        vec![
            (MemoryModuleType::WalkTarget, MemoryStatus::Registered),
            (memory, MemoryStatus::ValuePresent),
        ],
        SetWalkTargetAwayFrom {
            memory,
            speed_modifier,
            desired_distance: f64::from(desired_distance),
        },
    )
}

/// `LocateHidingPlace`: the home under the
/// villager, else any home within `radius`, else its own.
#[derive(Debug, Clone, Copy)]
pub struct LocateHidingPlace {
    radius: i32,
    speed_modifier: f32,
    close_enough: i32,
    next_attempt: i64,
}

impl BehaviorHooks for LocateHidingPlace {
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        context: &mut BrainTickContext,
        timestamp: i64,
    ) -> bool {
        if timestamp < self.next_attempt {
            return false;
        }
        self.next_attempt = timestamp + 20;
        let position = tick.world().body().position;
        let block = containing_block_pos(position);
        let close_enough = f64::from(self.close_enough);
        let nearby = tick
            .world_mut()
            .home_pois_within(block, self.close_enough + 1)
            .first()
            .copied()
            .filter(|pos| closer_to_center_than(*pos, position, close_enough));
        let hiding_place = nearby
            .or_else(|| {
                let homes = tick.world_mut().home_pois_within(block, self.radius);
                (!homes.is_empty()).then(|| {
                    let len = i32::try_from(homes.len()).unwrap_or(i32::MAX);
                    homes[usize::try_from(context.next_int(len)).unwrap_or(0)]
                })
            })
            .or_else(|| global_pos(tick, MemoryModuleType::Home).map(|home| home.pos));
        if let Some(pos) = hiding_place {
            hide_at(tick, pos, self.speed_modifier, self.close_enough);
        }
        true
    }

    fn debug_string(&self) -> &'static str {
        "LocateHidingPlace"
    }
}

fn hide_at(tick: &mut BrainTick<'_>, pos: BlockPos, speed_modifier: f32, close_enough: i32) {
    for memory in [
        MemoryModuleType::Path,
        MemoryModuleType::LookTarget,
        MemoryModuleType::BreedTarget,
        MemoryModuleType::InteractionTarget,
    ] {
        tick.erase_memory(memory);
    }
    let dimension = tick.world().dimension().to_owned();
    tick.set_memory_value(
        MemoryModuleType::HidingPlace,
        MemoryValue::GlobalPos(MemoryGlobalPos { dimension, pos }),
    );
    let position: Vec3d = tick.world().body().position;
    if !closer_to_center_than(pos, position, f64::from(close_enough)) {
        tick.set_memory_value(
            MemoryModuleType::WalkTarget,
            MemoryValue::WalkTarget(MemoryWalkTarget::at(pos, speed_modifier, close_enough)),
        );
    }
}

pub fn locate_hiding_place(radius: i32, speed_modifier: f32, close_enough: i32) -> Behavior {
    Behavior::one_shot(
        vec![
            (MemoryModuleType::WalkTarget, MemoryStatus::ValueAbsent),
            (MemoryModuleType::Home, MemoryStatus::Registered),
            (MemoryModuleType::HidingPlace, MemoryStatus::Registered),
            (MemoryModuleType::Path, MemoryStatus::Registered),
            (MemoryModuleType::LookTarget, MemoryStatus::Registered),
            (MemoryModuleType::BreedTarget, MemoryStatus::Registered),
            (
                MemoryModuleType::InteractionTarget,
                MemoryStatus::Registered,
            ),
        ],
        LocateHidingPlace {
            radius,
            speed_modifier,
            close_enough,
            next_attempt: 0,
        },
    )
}

/// `SetHiddenState`: a villager stays hidden
/// for `seconds` near its hiding place, or gives up 300 ticks after the
/// bell, and then forgets both; its activity update then leaves HIDE.
#[derive(Debug, Clone, Copy)]
pub struct SetHiddenState {
    stay_hidden_ticks: i32,
    close_enough: i32,
    ticks_hidden: i32,
}

impl BehaviorHooks for SetHiddenState {
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        timestamp: i64,
    ) -> bool {
        let Some(MemoryValue::Long(heard)) = tick.memory(MemoryModuleType::HeardBellTime) else {
            return false;
        };
        let timed_out = heard + 300 <= timestamp;
        if self.ticks_hidden <= self.stay_hidden_ticks && !timed_out {
            if let Some(hide) = global_pos(tick, MemoryModuleType::HidingPlace) {
                let body = containing_block_pos(tick.world().body().position);
                let (dx, dy, dz) = (
                    hide.pos.x - body.x,
                    hide.pos.y - body.y,
                    hide.pos.z - body.z,
                );
                if dx * dx + dy * dy + dz * dz < self.close_enough * self.close_enough {
                    self.ticks_hidden += 1;
                }
            }
        } else {
            tick.erase_memory(MemoryModuleType::HeardBellTime);
            tick.erase_memory(MemoryModuleType::HidingPlace);
            self.ticks_hidden = 0;
        }
        true
    }

    fn debug_string(&self) -> &'static str {
        "SetHiddenState"
    }
}

pub fn set_hidden_state(seconds: i32, close_enough: i32) -> Behavior {
    Behavior::one_shot(
        vec![
            (MemoryModuleType::HidingPlace, MemoryStatus::ValuePresent),
            (MemoryModuleType::HeardBellTime, MemoryStatus::ValuePresent),
        ],
        SetHiddenState {
            stay_hidden_ticks: seconds * 20,
            close_enough,
            ticks_hidden: 0,
        },
    )
}
