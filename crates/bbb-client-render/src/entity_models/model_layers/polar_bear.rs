use super::{apply_head_look, apply_quadruped_leg_swing, PartPose};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Which polar bear model part a standing-pose delta applies to. Vanilla
/// `PolarBearModel.setupAnim` rears the bear by moving the head, body, and both
/// front legs; the hind legs stay put.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::entity_models) enum PolarBearStandPart {
    Head,
    Body,
    FrontLeg,
}

/// The named parts the standing rear-up pose moves (head, body, both front legs), with their roles.
/// The swing/standing pose resolves by name, so this is the same for the adult and baby layouts.
pub(in crate::entity_models) const fn polar_bear_standing_part_roles(
) -> [(&'static str, PolarBearStandPart); 4] {
    [
        ("head", PolarBearStandPart::Head),
        ("body", PolarBearStandPart::Body),
        ("right_front_leg", PolarBearStandPart::FrontLeg),
        ("left_front_leg", PolarBearStandPart::FrontLeg),
    ]
}

/// Applies the vanilla `PolarBearModel.setupAnim` standing delta to one part
/// pose. `stand_scale` is `PolarBear.getStandingAnimationScale`; vanilla squares
/// it into `standScale`. `bodyAgeScale` (`state.ageScale`, 1.0 adult / 0.5 baby)
/// scales only the body and front-leg translation terms. Both front legs share
/// the same base y/z, so applying the delta to each matches vanilla's
/// `leftFrontLeg.y = rightFrontLeg.y` / `leftFrontLeg.z = rightFrontLeg.z`.
pub(in crate::entity_models) fn apply_polar_bear_standing_pose(
    pose: &mut PartPose,
    part: PolarBearStandPart,
    age_scale: f32,
    stand_scale: f32,
) {
    let scale = stand_scale * stand_scale;
    match part {
        PolarBearStandPart::Body => {
            pose.rotation[0] -= scale * std::f32::consts::PI * 0.35;
            pose.offset[1] += scale * age_scale * 2.0;
        }
        PolarBearStandPart::FrontLeg => {
            pose.offset[1] -= scale * age_scale * 20.0;
            pose.offset[2] += scale * age_scale * 4.0;
            pose.rotation[0] -= scale * std::f32::consts::PI * 0.45;
        }
        PolarBearStandPart::Head => {
            pose.offset[1] -= scale * 24.0;
            pose.offset[2] += scale * 13.0;
            pose.rotation[0] += scale * std::f32::consts::PI * 0.15;
        }
    }
}

/// Vanilla `PolarBearModel`.
pub(in crate::entity_models) fn polar_bear_model(
    baby: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = if baby {
        ModelLayer::POLAR_BEAR_BABY
    } else {
        ModelLayer::POLAR_BEAR
    }
    .bake();
    let render_state = &instance.render_state;
    apply_head_look(
        root.child_mut("head"),
        render_state.head_yaw,
        render_state.head_pitch,
    );
    apply_quadruped_leg_swing(
        &mut root,
        render_state.walk_animation_pos,
        render_state.walk_animation_speed,
    );
    let stand_scale = render_state.kind.polar_bear().stand_scale;
    if stand_scale != 0.0 {
        for (name, part) in polar_bear_standing_part_roles() {
            apply_polar_bear_standing_pose(
                &mut root.child_mut(name).pose,
                part,
                render_state.age_scale,
                stand_scale,
            );
        }
    }
    root
}
