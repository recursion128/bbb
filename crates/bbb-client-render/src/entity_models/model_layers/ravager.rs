use super::{
    head_look_at_rest, head_look_pose, limb_swing_at_rest, ravager_leg_swing_pose, triangle_wave,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::ModelPart;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// The four leg child names in `RavagerModel.createBodyLayer` order: the two hind legs then the two
/// front legs. [`ravager_leg_swing_pose`] resolves each leg's phase from its bind offset, so the
/// names only need to be distinct and render in the layer's order.
const RAVAGER_LEG_NAMES: [&str; 4] = [
    "right_hind_leg",
    "left_hind_leg",
    "right_front_leg",
    "left_front_leg",
];

/// Vanilla `RavagerModel.setupAnim`'s neck/mouth combat poses, driven by the projected attack / stun /
/// roar timers (the legs and head look are applied separately). While `attackTicksRemaining > 0` the
/// neck lunges forward (`neck.z = -6.5 + headPos`, `headPos = ((1 + triangleWave(tick, 10))·0.5)³·12`)
/// and the jaw snaps in two phases; otherwise the neck rests, the jaw cracks open `π·0.01`, and either a
/// stun tilts the neck (`xRot = 0.21991149`), side-shakes it (`neck.x = sin(stunned/40·10)·3`) and opens
/// the jaw `π·0.05`, or a post-stun roar gapes it (`mouth.xRot = π/2·sin(roar·π/4)`). Vanilla reads
/// `sin(neck.xRot)` for `yOffset` BEFORE setting `neck.xRot` (the reset bind `xRot` is `0`, so the term
/// is `0`); reading the live bind value keeps that faithful. Always applied — a resting ravager's jaw
/// sits at `π·0.01`.
fn apply_ravager_combat(
    root: &mut ModelPart,
    stunned_ticks: f32,
    attack_ticks: f32,
    roar_animation: f32,
) {
    use std::f32::consts::PI;
    let neck = root.child_mut("neck");
    if attack_ticks > 0.0 {
        let head_anim = triangle_wave(attack_ticks, 10.0);
        let scaled = (1.0 + head_anim) * 0.5;
        let head_pos = scaled * scaled * scaled * 12.0;
        let y_offset = head_pos * neck.pose.rotation[0].sin();
        neck.pose.offset[2] = -6.5 + head_pos;
        neck.pose.offset[1] = -7.0 - y_offset;
        let mouth = neck.child_mut("head").child_mut("mouth");
        mouth.pose.rotation[0] = if attack_ticks > 5.0 {
            ((-4.0 + attack_ticks) / 4.0).sin() * PI * 0.4
        } else {
            (PI / 20.0) * (PI * attack_ticks / 10.0).sin()
        };
    } else {
        let y_offset = -neck.pose.rotation[0].sin();
        neck.pose.offset[0] = 0.0;
        neck.pose.offset[1] = -7.0 - y_offset;
        neck.pose.offset[2] = 5.5;
        let is_stunned = stunned_ticks > 0.0;
        neck.pose.rotation[0] = if is_stunned { 0.21991149 } else { 0.0 };
        if is_stunned {
            let speed = f64::from(stunned_ticks) / 40.0;
            neck.pose.offset[0] = (speed * 10.0).sin() as f32 * 3.0;
        }
        let mouth = neck.child_mut("head").child_mut("mouth");
        mouth.pose.rotation[0] = if is_stunned {
            PI * 0.05
        } else if roar_animation > 0.0 {
            (PI / 2.0) * (roar_animation * PI * 0.25).sin()
        } else {
            PI * 0.01
        };
    }
}

/// Mutable ravager model, mirroring vanilla `RavagerModel`. The unified tree is built once with the
/// vanilla `RavagerModel.createBodyLayer` child names: `neck` (parenting `head` → horns/mouth),
/// `body`, and the four legs. `setup_anim` applies the neck/mouth attack/stun/roar combat poses
/// ([`apply_ravager_combat`]), swings the four legs ([`ravager_leg_swing_pose`]) and looks the head —
/// which, being nested under the neck, is reached as `neck.child_mut("head")` so its horn/mouth
/// descendants inherit the look automatically.
pub(in crate::entity_models) fn ravager_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::RAVAGER.bake();
    let render_state = &instance.render_state;
    // Vanilla `RavagerModel.setupAnim` runs the neck/mouth combat poses first (always — the resting
    // jaw cracks open `π·0.01`), then the head look and leg swing.
    apply_ravager_combat(
        &mut root,
        render_state.kind.ravager().stunned_ticks_remaining,
        render_state.kind.ravager().attack_ticks_remaining,
        render_state.kind.ravager().roar_animation,
    );
    if !limb_swing_at_rest(render_state.walk_animation_speed) {
        for name in RAVAGER_LEG_NAMES {
            let leg = root.child_mut(name);
            leg.pose = ravager_leg_swing_pose(
                leg.pose,
                render_state.walk_animation_pos,
                render_state.walk_animation_speed,
            );
        }
    }
    if !head_look_at_rest(render_state.head_yaw, render_state.head_pitch) {
        let head = root.child_mut("neck").child_mut("head");
        head.pose = head_look_pose(head.pose, render_state.head_yaw, render_state.head_pitch);
    }
    root
}
