//! The villager's sleep: the REST package's bed and the CORE package's
//! wake-up.

use bbb_protocol::packets::BlockPos;

use super::super::effect::BrainWorldEffect;
use super::super::memory::{MemoryModuleType, MemoryStatus, MemoryValue, MemoryWalkTarget};
use super::super::{Activity, Behavior, BehaviorHooks, BrainTick, BrainTickContext};
use super::containing_block_pos;
use super::villager_work::{closer_to_center_than, global_pos, in_this_level};
use crate::entity::ai::mob::VILLAGER_REQUIRED_PATH_LENGTH;

/// `SleepInBed.COOLDOWN_AFTER_BEING_WOKEN` (`SleepInBed.java:20`).
const COOLDOWN_AFTER_BEING_WOKEN: i64 = 100;

fn sleeping(tick: &BrainTick<'_>) -> bool {
    tick.world()
        .body()
        .villager
        .is_some_and(|villager| villager.sleeping)
}

fn home(tick: &BrainTick<'_>) -> Option<BlockPos> {
    global_pos(tick, MemoryModuleType::Home)
        .filter(|home| in_this_level(tick, home))
        .map(|home| home.pos)
}

/// `Villager.stopSleeping`; `LevelCtx::stop_sleeping`
/// records `LAST_WOKEN`.
fn stop_sleeping(tick: &mut BrainTick<'_>) {
    tick.world_mut()
        .queue_effect(BrainWorldEffect::StopSleeping);
}

/// `SleepInBed`. The body only lies down once the
/// tick's effects replay, so the tick that started the sleep answers for the
/// bed position the body is about to take.
#[derive(Debug, Clone, Copy, Default)]
pub struct SleepInBed {
    next_ok_start_time: i64,
    lay_down_at: i64,
}

impl BehaviorHooks for SleepInBed {
    fn extra_start_conditions(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
    ) -> bool {
        let Some(bed) = home(tick) else {
            return false;
        };
        if let Some(MemoryValue::Long(woken)) = tick.memory(MemoryModuleType::LastWoken) {
            let since = tick.world().game_time() - woken;
            if since > 0 && since < COOLDOWN_AFTER_BEING_WOKEN {
                return false;
            }
        }
        closer_to_center_than(bed, tick.world().body().position, 2.0)
            && tick.world().bed_occupied(bed) == Some(false)
    }

    fn start(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, timestamp: i64) {
        if timestamp <= self.next_ok_start_time {
            return;
        }
        let Some(bed) = home(tick) else {
            return;
        };
        tick.world_mut()
            .queue_effect(BrainWorldEffect::StartSleeping { pos: bed });
        self.lay_down_at = timestamp;
        tick.set_memory_value(MemoryModuleType::LastSlept, MemoryValue::Long(timestamp));
        tick.erase_memory(MemoryModuleType::WalkTarget);
        tick.erase_memory(MemoryModuleType::CantReachWalkTargetSince);
    }

    fn can_still_use(
        &self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        timestamp: i64,
    ) -> bool {
        let Some(bed) = global_pos(tick, MemoryModuleType::Home).map(|home| home.pos) else {
            return false;
        };
        if !tick.is_active(Activity::REST) {
            return false;
        }
        if self.lay_down_at == timestamp {
            return true;
        }
        let position = tick.world().body().position;
        position.y > f64::from(bed.y) + 0.4 && closer_to_center_than(bed, position, 1.14)
    }

    fn stop(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, timestamp: i64) {
        if sleeping(tick) || self.lay_down_at == timestamp {
            stop_sleeping(tick);
            self.next_ok_start_time = timestamp + 40;
        }
    }

    fn timed_out(&self, _timestamp: i64, _end_timestamp: i64) -> bool {
        false
    }

    fn debug_string(&self) -> &'static str {
        "SleepInBed"
    }
}

pub fn sleep_in_bed() -> Behavior {
    Behavior::new(
        vec![
            (MemoryModuleType::Home, MemoryStatus::ValuePresent),
            (MemoryModuleType::LastWoken, MemoryStatus::Registered),
            (MemoryModuleType::LastSlept, MemoryStatus::Registered),
            (MemoryModuleType::WalkTarget, MemoryStatus::Registered),
            (
                MemoryModuleType::CantReachWalkTargetSince,
                MemoryStatus::Registered,
            ),
        ],
        SleepInBed::default(),
    )
}

/// `WakeUp`.
#[derive(Debug, Clone, Copy)]
pub struct WakeUp;

impl BehaviorHooks for WakeUp {
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        if tick.is_active(Activity::REST) || !sleeping(tick) {
            return false;
        }
        stop_sleeping(tick);
        true
    }

    fn debug_string(&self) -> &'static str {
        "WakeUp"
    }
}

pub fn wake_up() -> Behavior {
    Behavior::one_shot(Vec::new(), WakeUp)
}

/// `SetClosestHomeAsWalkTarget`:
/// a bedless villager walks to the nearest reachable bed, trying at most four
/// new beds per attempt and skipping the ones tried in the last 40 ticks.
#[derive(Debug, Clone, Default)]
struct SetClosestHomeAsWalkTarget {
    speed: f32,
    last_update: i64,
    tried: Vec<(BlockPos, i64)>,
}

impl BehaviorHooks for SetClosestHomeAsWalkTarget {
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        let game_time = tick.world().game_time();
        if game_time - self.last_update < 20 {
            return false;
        }
        self.last_update = game_time + i64::from(tick.world_mut().level_random_next_int(20));
        let body = containing_block_pos(tick.world().body().position);
        let homes = tick.world_mut().home_pois_within(body, 48);
        let Some(closest) = homes.first() else {
            return false;
        };
        let (dx, dy, dz) = (closest.x - body.x, closest.y - body.y, closest.z - body.z);
        if dx * dx + dy * dy + dz * dz <= 4 {
            return false;
        }
        let last_update = self.last_update;
        self.tried.retain(|(_, until)| *until >= last_update);
        let fresh: Vec<_> = homes
            .into_iter()
            .filter(|pos| !self.tried.iter().any(|(tried, _)| tried == pos))
            .take(4)
            .collect();
        self.tried
            .extend(fresh.iter().map(|pos| (*pos, last_update + 40)));
        let reachable = tick
            .world()
            .body_path_to_pois(&fresh, 1, VILLAGER_REQUIRED_PATH_LENGTH)
            .and_then(|(pos, reached)| reached.then_some(pos));
        if let Some(pos) = reachable {
            tick.set_memory_value(
                MemoryModuleType::WalkTarget,
                MemoryValue::WalkTarget(MemoryWalkTarget::at(pos, self.speed, 1)),
            );
        }
        true
    }

    fn debug_string(&self) -> &'static str {
        "SetClosestHomeAsWalkTarget"
    }
}

pub fn set_closest_home_as_walk_target(speed: f32) -> Behavior {
    Behavior::one_shot(
        vec![
            (MemoryModuleType::WalkTarget, MemoryStatus::ValueAbsent),
            (MemoryModuleType::Home, MemoryStatus::ValueAbsent),
        ],
        SetClosestHomeAsWalkTarget {
            speed,
            ..SetClosestHomeAsWalkTarget::default()
        },
    )
}

/// `InsideBrownianWalk`: indoors, step to a
/// random neighbouring block that is also roofed and can be stood on.
#[derive(Debug, Clone, Copy)]
struct InsideBrownianWalk {
    speed: f32,
}

impl BehaviorHooks for InsideBrownianWalk {
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        let body = containing_block_pos(tick.world().body().position);
        if tick.world().can_see_sky(body) {
            return false;
        }
        let mut cells: Vec<BlockPos> = (-1..=1)
            .flat_map(|x| {
                (-1..=1).flat_map(move |y| {
                    (-1..=1).map(move |z| BlockPos {
                        x: body.x + x,
                        y: body.y + y,
                        z: body.z + z,
                    })
                })
            })
            .collect();
        for i in (1..cells.len()).rev() {
            let j = context.next_int(i32::try_from(i + 1).unwrap_or(i32::MAX));
            cells.swap(i, usize::try_from(j).unwrap_or(0));
        }
        let target = cells.into_iter().find(|pos| {
            let below = BlockPos {
                y: pos.y - 1,
                ..*pos
            };
            !tick.world().can_see_sky(*pos)
                && tick
                    .world()
                    .block_facts(*pos)
                    .is_some_and(|facts| facts.collision_shape_empty)
                && tick
                    .world()
                    .block_facts(below)
                    .is_some_and(|facts| facts.face_sturdy_up)
        });
        if let Some(pos) = target {
            tick.set_memory_value(
                MemoryModuleType::WalkTarget,
                MemoryValue::WalkTarget(MemoryWalkTarget::at(pos, self.speed, 0)),
            );
        }
        true
    }

    fn debug_string(&self) -> &'static str {
        "InsideBrownianWalk"
    }
}

pub fn inside_brownian_walk(speed: f32) -> Behavior {
    Behavior::one_shot(
        vec![(MemoryModuleType::WalkTarget, MemoryStatus::ValueAbsent)],
        InsideBrownianWalk { speed },
    )
}
