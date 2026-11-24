use super::{humanoid_leg_swing_pose, limb_swing_at_rest};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::BakedRoot;
use crate::entity_models::ChickenModelVariant;

/// Vanilla `ChickenModel`.
pub(in crate::entity_models) fn chicken_model(
    variant: ChickenModelVariant,
    baby: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = crate::entity_models::textured::chicken_model_layer(variant, baby).bake();
    let render_state = &instance.render_state;
    // Vanilla `ChickenModel.setupAnim`: `flapAngle = (sin(flap) + 1) * flapSpeed`,
    // applied as `rightWing.zRot = flapAngle` / `leftWing.zRot = -flapAngle`. The
    // adult, cold, and headless-baby layers all carry the named wings. With
    // `flapSpeed == 0` (a grounded/still chicken) the angle is `0`, so the wings
    // hold the bind pose.
    let flap_angle =
        (render_state.kind.chicken().flap.sin() + 1.0) * render_state.kind.chicken().flap_speed;
    root.child_mut("right_wing").pose.rotation[2] = flap_angle;
    root.child_mut("left_wing").pose.rotation[2] = -flap_angle;
    // `humanoid_leg_swing_pose` resolves each leg's phase from its `x` sign, so the named legs work
    // for the adult, cold, and headless-baby layouts alike. A standing chicken leaves the legs at
    // their bind pose.
    if !limb_swing_at_rest(render_state.walk_animation_speed) {
        for name in ["right_leg", "left_leg"] {
            let leg = root.child_mut(name);
            leg.pose = humanoid_leg_swing_pose(
                leg.pose,
                render_state.walk_animation_pos,
                render_state.walk_animation_speed,
            );
        }
    }
    root
}
