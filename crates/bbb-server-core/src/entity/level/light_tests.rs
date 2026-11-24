use super::*;
use crate::entity::tests::native_level::NativeLevel;

const POS: BlockPos = BlockPos { x: 8, y: 70, z: 8 };

#[test]
fn native_daylight_changes_the_light_cost_used_by_strolling() {
    let mut native = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::console_command(&sender, &receiver, &mut native.world, 1, "time set 6000");
    let daylight = native.with_level(|level| {
        assert!(level.max_local_raw_brightness(POS).is_some());
        assert!(level.dimension_ambient_light().is_some());
        assert!(crate::entity::ai::mob::stroll::level_weighs_positions(
            level
        ));
        pathfinding_cost_from_light_levels(level, POS).unwrap()
    });
    crate::tests::console_command(&sender, &receiver, &mut native.world, 2, "time set 18000");
    let night = native.with_level(|level| pathfinding_cost_from_light_levels(level, POS).unwrap());
    assert!(daylight > night);
}
