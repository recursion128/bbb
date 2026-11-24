use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla `PhantomModel`.
pub(in crate::entity_models) fn phantom_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::PHANTOM.bake();
    // Vanilla `PhantomModel.setupAnim` flap: each wing's base and tip `zRot` is set to
    // ±[`phantom_wing_z_rot`] (left positive, right negated) and each tail segment's `xRot` to
    // [`phantom_tail_x_rot`], overwriting the rest dihedral every frame. The head holds its rest
    // tilt. The flap always advances (`flapTime` tracks `ageInTicks`), so this runs unconditionally.
    let flap = phantom_flap_time(instance.entity_id, instance.render_state.age_in_ticks);
    let wing_z = phantom_wing_z_rot(flap);
    let tail_x = phantom_tail_x_rot(flap);
    let body = root.child_mut("body");

    let tail_base = body.child_mut("tail_base");
    tail_base.pose.rotation[0] = tail_x;
    tail_base.child_mut("tail_tip").pose.rotation[0] = tail_x;

    let left_base = body.child_mut("left_wing_base");
    left_base.pose.rotation[2] = wing_z;
    left_base.child_mut("left_wing_tip").pose.rotation[2] = wing_z;

    let right_base = body.child_mut("right_wing_base");
    right_base.pose.rotation[2] = -wing_z;
    right_base.child_mut("right_wing_tip").pose.rotation[2] = -wing_z;
    root
}

/// Vanilla `PhantomRenderer.extractRenderState`: `flapTime = getUniqueFlapTickOffset() +
/// ageInTicks`, where `Phantom.getUniqueFlapTickOffset() = getId() * 3` — a deterministic
/// per-entity phase offset (Java `int` multiply, so it wraps like vanilla) plus the projected
/// `ageInTicks`.
pub(in crate::entity_models) fn phantom_flap_time(entity_id: i32, age_in_ticks: f32) -> f32 {
    entity_id.wrapping_mul(3) as f32 + age_in_ticks
}

/// Vanilla `PhantomModel.setupAnim` flap phase: `anim = flapTime * FLAP_DEGREES_PER_TICK *
/// π/180`, where `FLAP_DEGREES_PER_TICK = 7.448451`. Returned in radians.
fn phantom_flap_anim(flap_time: f32) -> f32 {
    (flap_time * 7.448451).to_radians()
}

/// Vanilla `PhantomModel.setupAnim` left-wing `zRot = cos(anim) * 16°`. The right wing uses
/// the negation. Both the base and tip wing parts share this value (set absolutely, so the
/// rest `±0.1` dihedral is overwritten every frame).
pub(in crate::entity_models) fn phantom_wing_z_rot(flap_time: f32) -> f32 {
    phantom_flap_anim(flap_time).cos() * 16.0_f32.to_radians()
}

/// Vanilla `PhantomModel.setupAnim` tail `xRot = -(5° + cos(2·anim) * 5°)`. Both the base and
/// tip tail parts share this value (set absolutely over the zeroed rest tail pose).
pub(in crate::entity_models) fn phantom_tail_x_rot(flap_time: f32) -> f32 {
    let anim = phantom_flap_anim(flap_time);
    -(5.0 + (anim * 2.0).cos() * 5.0).to_radians()
}
