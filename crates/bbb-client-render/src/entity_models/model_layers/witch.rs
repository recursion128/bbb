use super::{apply_half_amplitude_leg_swing, apply_head_look, witch_nose_bob_pose};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Mutable witch model, mirroring vanilla `WitchModel`. The unified tree is built with the vanilla
/// child names: `head` parents the hat chain, the nose, and the mole; `body` parents the jacket; the
/// combined `arms` and the two legs hang off the root. `setup_anim` looks the head ([`apply_head_look`]
/// on `head`), swings the legs at the villager-family half amplitude
/// ([`apply_half_amplitude_leg_swing`]), then bobs the nose continuously ([`witch_nose_bob_pose`],
/// driven by `ageInTicks` and the entity id), and finally applies the `isHoldingItem` nose hold pose —
/// reached as the head's `nose` child so it inherits the head look.
pub(in crate::entity_models) fn witch_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::WITCH.bake();
    let render_state = &instance.render_state;
    apply_head_look(
        root.child_mut("head"),
        render_state.head_yaw,
        render_state.head_pitch,
    );
    apply_half_amplitude_leg_swing(
        &mut root,
        render_state.walk_animation_pos,
        render_state.walk_animation_speed,
    );
    let nose = root.child_mut("head").child_mut("nose");
    nose.pose = witch_nose_bob_pose(nose.pose, render_state.age_in_ticks, instance.entity_id);
    if render_state.kind.witch().holding_item {
        nose.pose.offset = [0.0, 1.0, -1.5];
        nose.pose.rotation[0] = -0.9;
    }
    root
}
