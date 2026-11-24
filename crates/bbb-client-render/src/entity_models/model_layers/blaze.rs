use super::{head_look_at_rest, head_look_pose};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// The number of rods in the blaze body layer (parts `1..=12`; part `0` is the head).
pub(in crate::entity_models) const BLAZE_ROD_COUNT: usize = 12;

/// Vanilla `BlazeModel.getPartName`.
const BLAZE_ROD_NAMES: [&str; BLAZE_ROD_COUNT] = [
    "part0", "part1", "part2", "part3", "part4", "part5", "part6", "part7", "part8", "part9",
    "part10", "part11",
];

/// Vanilla `BlazeModel.setupAnim` rod placement: the twelve rods orbit in three rings of
/// four, their `x`/`y`/`z` offsets SET every frame from `ageInTicks`. Ring 0 (rods 0..4) at
/// radius 9, ring 1 (4..8) at radius 7, ring 2 (8..12) at radius 5; each ring spins at its
/// own rate and the rods bob in `y`. Returns the part offset for rod `index`.
pub(in crate::entity_models) fn blaze_rod_offset(index: usize, age_in_ticks: f32) -> [f32; 3] {
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};
    let i = index as f32;
    let (radius, y, base_angle) = if index < 4 {
        (
            9.0,
            -2.0 + ((2.0 * i + age_in_ticks) * 0.25).cos(),
            age_in_ticks * PI * -0.1,
        )
    } else if index < 8 {
        (
            7.0,
            2.0 + ((2.0 * i + age_in_ticks) * 0.25).cos(),
            FRAC_PI_4 + age_in_ticks * PI * 0.03,
        )
    } else {
        (
            5.0,
            11.0 + ((1.5 * i + age_in_ticks) * 0.5).cos(),
            0.47123894 + age_in_ticks * PI * -0.05,
        )
    };
    let angle = base_angle + (index % 4) as f32 * FRAC_PI_2;
    [angle.cos() * radius, y, angle.sin() * radius]
}

/// Vanilla `BlazeModel`.
pub(in crate::entity_models) fn blaze_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::BLAZE.bake();
    let age_in_ticks = instance.render_state.age_in_ticks;
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    if !head_look_at_rest(head_yaw, head_pitch) {
        let head = root.child_mut("head");
        head.pose = head_look_pose(head.pose, head_yaw, head_pitch);
    }
    for (index, &name) in BLAZE_ROD_NAMES.iter().enumerate() {
        root.child_mut(name).pose.offset = blaze_rod_offset(index, age_in_ticks);
    }
    root
}
