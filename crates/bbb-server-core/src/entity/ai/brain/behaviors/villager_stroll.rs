//! Wandering that leads villagers back towards an occupied village.

use super::super::{Behavior, BehaviorHooks, BrainTick, BrainTickContext};
use super::{make_walk_target, MemoryModuleType, MemoryStatus, MemoryValue};
use bbb_protocol::packets::Vec3d;

#[derive(Clone)]
struct VillageBoundRandomStroll {
    speed: f32,
    horizontal_dist: i32,
    vertical_dist: i32,
    close_enough: i32,
    /// `GoToClosestVillage`: only outside
    /// a village.
    outside_only: bool,
    next_attempt: i64,
}

impl BehaviorHooks for VillageBoundRandomStroll {
    // `VillageBoundRandomStroll.create`.
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
        let position = tick.world().body().block_position;
        let in_village = tick.world().is_village(position);
        if in_village && self.outside_only {
            return false;
        }
        let towards = if in_village {
            None
        } else {
            tick.world().closest_village_section(position)
        };
        let sampled = if let Some(center) = towards {
            tick.world_mut().body_default_random_pos_towards(
                context.body_random(),
                self.horizontal_dist,
                self.vertical_dist,
                Vec3d {
                    x: f64::from(center.x) + 0.5,
                    y: f64::from(center.y),
                    z: f64::from(center.z) + 0.5,
                },
                std::f64::consts::FRAC_PI_2,
            )
        } else {
            tick.world_mut().body_land_random_pos(
                context.body_random(),
                self.horizontal_dist,
                self.vertical_dist,
            )
        };
        let Some(sampled) = sampled else {
            return false;
        };
        match sampled {
            Some(pos) => tick.set_memory_value(
                MemoryModuleType::WalkTarget,
                MemoryValue::WalkTarget(make_walk_target(pos, self.speed, self.close_enough)),
            ),
            None if self.outside_only => {}
            None => tick.erase_memory(MemoryModuleType::WalkTarget),
        }
        true
    }
    fn debug_string(&self) -> &'static str {
        if self.outside_only {
            "GoToClosestVillage"
        } else {
            "VillageBoundRandomStroll"
        }
    }
}

pub fn village_bound_random_stroll(speed: f32) -> Behavior {
    village_bound_random_stroll_within(speed, 10, 7)
}

pub fn village_bound_random_stroll_within(
    speed: f32,
    horizontal_dist: i32,
    vertical_dist: i32,
) -> Behavior {
    Behavior::one_shot(
        vec![(MemoryModuleType::WalkTarget, MemoryStatus::ValueAbsent)],
        VillageBoundRandomStroll {
            speed,
            horizontal_dist,
            vertical_dist,
            close_enough: 0,
            outside_only: false,
            next_attempt: 0,
        },
    )
}

/// `GoToClosestVillage.create(speed, 4)`; the section scoring is the same
/// walk towards the closest village section the stroll takes.
pub fn go_to_closest_village(speed: f32) -> Behavior {
    Behavior::one_shot(
        vec![(MemoryModuleType::WalkTarget, MemoryStatus::ValueAbsent)],
        VillageBoundRandomStroll {
            speed,
            horizontal_dist: 15,
            vertical_dist: 7,
            close_enough: 4,
            outside_only: true,
            next_attempt: 0,
        },
    )
}

#[derive(Clone, Default)]
struct MoveToSkySeeingSpot {
    next_attempt: i64,
}

impl BehaviorHooks for MoveToSkySeeingSpot {
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
        let Some(pos) = tick
            .world_mut()
            .body_outdoor_position(context.body_random())
        else {
            return false;
        };
        tick.set_memory_value(
            MemoryModuleType::WalkTarget,
            MemoryValue::WalkTarget(make_walk_target(pos, 0.5, 0)),
        );
        true
    }
    fn debug_string(&self) -> &'static str {
        "MoveToSkySeeingSpot"
    }
}

pub fn move_to_sky_seeing_spot() -> Behavior {
    Behavior::one_shot(
        vec![(MemoryModuleType::WalkTarget, MemoryStatus::ValueAbsent)],
        MoveToSkySeeingSpot::default(),
    )
}
