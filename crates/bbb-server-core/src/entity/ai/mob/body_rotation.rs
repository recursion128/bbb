pub(super) use bbb_protocol::entity_body_rotation::{
    base_body_rotation_max_yaw, BodyRotationControl,
};

use super::spec::mob_spec;
use crate::entity::living::LivingEntityState;

// Vanilla normalizes the retained body/head rotations immediately after
// `tickHeadTurn`.
pub(super) fn normalize_retained_rotations(living: &mut LivingEntityState) {
    while living.y_body_rot - living.y_body_rot_o < -180.0 {
        living.y_body_rot_o -= 360.0;
    }
    while living.y_body_rot - living.y_body_rot_o >= 180.0 {
        living.y_body_rot_o += 360.0;
    }
    while living.y_head_rot - living.y_head_rot_o < -180.0 {
        living.y_head_rot_o -= 360.0;
    }
    while living.y_head_rot - living.y_head_rot_o >= 180.0 {
        living.y_head_rot_o += 360.0;
    }
}

// `Mob.getMaxHeadXRot` and its mob overrides; Wolf reads a sitting pose at tick time.
pub(super) fn base_max_head_x_rot(entity_type_id: i32) -> Option<f32> {
    mob_spec(entity_type_id).map_or(Some(40.0), |spec| spec.max_head_x_rot)
}

#[cfg(test)]
mod tests;
