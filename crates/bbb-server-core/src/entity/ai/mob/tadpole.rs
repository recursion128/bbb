//! Tadpole brain behavior packages.

use crate::entity::ai::brain::{ordered_try_all, random_stroll_swim, trigger_if_in_water};
use crate::entity::ai::{
    animal_panic, count_down_cooldown_ticks, follow_temptation, look_at_target_sink,
    move_to_target_sink, set_entity_look_target_sometimes_of, set_walk_target_from_look_target,
    Activity, Brain, BrainPacked, MemoryModuleType, MemoryStatus, SensorType,
};

// Tadpole; SensorType binds its temptation sensor to frog food.
pub(super) fn tadpole_brain(packed: &BrainPacked) -> Brain {
    let mut brain = Brain::new();
    brain.register_sensor(SensorType::NearestLivingEntities);
    brain.register_sensor(SensorType::NearestPlayers);
    brain.register_sensor(SensorType::HurtBy);
    brain.register_sensor(SensorType::FrogTemptations);
    brain.add_activity(
        Activity::CORE,
        [
            (0, animal_panic(2.0)),
            (1, look_at_target_sink()),
            (2, move_to_target_sink()),
            (
                3,
                count_down_cooldown_ticks(MemoryModuleType::TemptationCooldownTicks),
            ),
        ],
        &[],
        &[],
    );
    brain.add_activity(
        Activity::IDLE,
        [
            (
                0,
                set_entity_look_target_sometimes_of(Some("minecraft:player"), 6.0, 30, 60),
            ),
            (1, follow_temptation(1.25)),
            (
                2,
                ordered_try_all(
                    vec![(MemoryModuleType::WalkTarget, MemoryStatus::ValueAbsent)],
                    [
                        (2, random_stroll_swim(0.5)),
                        (3, set_walk_target_from_look_target(0.5, 3)),
                        (5, trigger_if_in_water()),
                    ],
                ),
            ),
        ],
        &[],
        &[],
    );
    brain.apply_packed(packed);
    brain
}
