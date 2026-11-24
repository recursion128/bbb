use crate::entity_models::catalog::CamelModelFamily;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::keyframe::{animate, animate_walk, AnimationDefinition};
use crate::entity_models::model::ModelPart;
use crate::entity_models::model_data_26_1::{camel_animation, camel_baby_animation};
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla `CamelModel.applyHeadRotation`: the net head look clamps to `yRot ∈ [-30, 30]` and
/// `xRot ∈ [-25, 45]`, then the post-dash `jumpCooldown` adds `45 * cooldown / 55` pitch before
/// a second `[-25, 70]` clamp. Returns the clamped `(yaw, pitch)` in degrees.
pub(in crate::entity_models) fn camel_clamped_head_look(
    head_yaw_deg: f32,
    head_pitch_deg: f32,
    jump_cooldown: f32,
) -> (f32, f32) {
    let yaw = head_yaw_deg.clamp(-30.0, 30.0);
    let mut pitch = head_pitch_deg.clamp(-25.0, 45.0);
    if jump_cooldown > 0.0 {
        pitch = (pitch + 45.0 * jump_cooldown / 55.0).clamp(-25.0, 70.0);
    }
    (yaw, pitch)
}

/// Vanilla `CamelModel` animation set: walk, sit, sit pose, stand up, idle, dash.
type CamelAnimations = [&'static AnimationDefinition; 6];

const ADULT_ANIMATIONS: CamelAnimations = [
    &camel_animation::CAMEL_WALK,
    &camel_animation::CAMEL_SIT,
    &camel_animation::CAMEL_SIT_POSE,
    &camel_animation::CAMEL_STANDUP,
    &camel_animation::CAMEL_IDLE,
    &camel_animation::CAMEL_DASH,
];

const BABY_ANIMATIONS: CamelAnimations = [
    &camel_baby_animation::CAMEL_BABY_WALK,
    &camel_baby_animation::CAMEL_BABY_SIT,
    &camel_baby_animation::CAMEL_BABY_SIT_POSE,
    &camel_baby_animation::CAMEL_BABY_STANDUP,
    &camel_baby_animation::CAMEL_BABY_IDLE,
    &camel_baby_animation::CAMEL_BABY_DASH,
];

/// Vanilla `AdultCamelModel` / `BabyCamelModel` / `CamelSaddleModel`.
pub(in crate::entity_models) fn camel_model(
    family: CamelModelFamily,
    baby: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    // `CamelHuskRenderer` has no baby model.
    let (layer, animations) = if family == CamelModelFamily::Camel && baby {
        (ModelLayer::CAMEL_BABY, &BABY_ANIMATIONS)
    } else {
        (ModelLayer::CAMEL, &ADULT_ANIMATIONS)
    };
    let mut root = layer.bake();
    let saddle = false;
    pose_camel(&mut root, animations, saddle, instance);
    root
}

pub(in crate::entity_models) fn camel_saddle_model(
    layer: ModelLayer,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = layer.bake();
    let animations = &ADULT_ANIMATIONS;
    let saddle = true;
    pose_camel(&mut root, animations, saddle, instance);
    root
}

fn pose_camel(
    root: &mut ModelPart,
    animations: &CamelAnimations,
    saddle: bool,
    instance: &EntityModelInstance,
) {
    let state = &instance.render_state;
    let (yaw, pitch) = camel_clamped_head_look(
        state.head_yaw,
        state.head_pitch,
        state.kind.camel().jump_cooldown,
    );
    let head = root.part_mut("head").unwrap();
    head.pose.rotation[0] = pitch.to_radians();
    head.pose.rotation[1] = yaw.to_radians();
    let [walk, sit, sit_pose, standup, idle, dash] = *animations;
    animate_walk(
        root,
        walk,
        state.walk_animation_pos,
        state.walk_animation_speed,
        2.0,
        2.5,
    );
    for (definition, seconds) in [
        (sit, state.kind.camel().sit_seconds),
        (sit_pose, state.kind.camel().sit_pose_seconds),
        (standup, state.kind.camel().standup_seconds),
        (idle, state.kind.camel().idle_seconds),
        (dash, state.kind.camel().dash_seconds),
    ] {
        if seconds >= 0.0 {
            animate(root, definition, seconds, 1.0);
        }
    }
    if saddle {
        root.part_mut("reins").unwrap().visible = state.kind.camel().saddle_ridden;
    }
}
