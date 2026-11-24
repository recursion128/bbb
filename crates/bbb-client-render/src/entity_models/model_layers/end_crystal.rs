use glam::{EulerRot, Quat, Vec3};

use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::ModelPart;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla `EndCrystalRenderer.getY(timeInTicks)`: the hover height that drives both the glass bob
/// and the beam offset. `hh = sin(t·0.2)/2 + 0.5; hh = (hh² + hh)·0.4; return hh − 1.4`.
pub(in crate::entity_models) fn end_crystal_get_y(age_in_ticks: f32) -> f32 {
    let hh = (age_in_ticks * 0.2).sin() / 2.0 + 0.5;
    let hh = (hh * hh + hh) * 0.4;
    hh - 1.4
}

/// Vanilla `EndCrystalModel.setupAnim`: `outerGlass.y += getY(age)·16 / 2`, so the whole glass stack
/// (outer/inner/core) bobs by `getY(age)·8` model-pixels. Negative (the crystal hovers above the
/// base) and applied to the shared `(0, 24, 0)` glass centre.
pub(in crate::entity_models) fn end_crystal_bob_y(age_in_ticks: f32) -> f32 {
    end_crystal_get_y(age_in_ticks) * 16.0 / 2.0
}

/// Vanilla `EndCrystalModel.setupAnim` glass spin, as the two local `rotateBy` quaternions. With
/// `TILT` = π/3 about the `(sin45, 0, sin45)` diagonal and `spin` = `Ry(age·3°)`:
/// - `outer_glass` rotates by `Ry(age·3°)·TILT` (vanilla `Axis.YP.rotationDegrees(speed).rotateAxis(π/3, …)`),
/// - `inner_glass` and `cube` each rotate by `TILT·Ry(age·3°)` (vanilla `setAngleAxis(π/3, …).rotateY(speed)`).
///
/// Returns `(outer, inner_and_core)`. The renderer nests them (`outer → inner → core`) so the inner
/// and core inherit the outer's rotation, exactly as vanilla's part hierarchy does. Even at age 0
/// the `TILT` leaves the crystal canted on the diagonal axis.
pub(in crate::entity_models) fn end_crystal_glass_quaternions(age_in_ticks: f32) -> (Quat, Quat) {
    use std::f32::consts::{FRAC_1_SQRT_2, FRAC_PI_3};
    let axis = Vec3::new(FRAC_1_SQRT_2, 0.0, FRAC_1_SQRT_2);
    let tilt = Quat::from_axis_angle(axis, FRAC_PI_3);
    let spin = Quat::from_rotation_y((age_in_ticks * 3.0).to_radians());
    (spin * tilt, tilt * spin)
}

/// Vanilla `EndCrystalModel`.
pub(in crate::entity_models) fn end_crystal_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::END_CRYSTAL.bake();
    let state = &instance.render_state;
    root.child_mut("base").visible = state.kind.end_crystal().shows_bottom;
    let (outer_rotation, inner_rotation) = end_crystal_glass_quaternions(state.age_in_ticks);
    let outer = root.child_mut("outer_glass");
    outer.pose.offset[1] += end_crystal_bob_y(state.age_in_ticks);
    rotate_by(outer, outer_rotation);
    let inner = outer.child_mut("inner_glass");
    rotate_by(inner, inner_rotation);
    rotate_by(inner.child_mut("cube"), inner_rotation);
    root
}

/// Vanilla `ModelPart.rotateBy`: composes `rotation` onto the part's ZYX Euler rotation.
fn rotate_by(part: &mut ModelPart, rotation: Quat) {
    let [x, y, z] = part.pose.rotation;
    let old = Quat::from_euler(EulerRot::ZYX, z, y, x);
    let (z, y, x) = (old * rotation).to_euler(EulerRot::ZYX);
    part.pose.rotation = [x, y, z];
}
