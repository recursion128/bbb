pub use bbb_render_types::SheepHeadEatPose;

use super::{apply_quadruped_leg_swing, PartPose};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::ModelPart;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Returns `true` when the sheep head is fully at rest — not eating and with no
/// head-look turn — so callers can borrow the static parts unchanged instead of
/// cloning to apply [`sheep_head_pose`].
pub(in crate::entity_models) fn sheep_head_at_rest(
    head_eat: SheepHeadEatPose,
    head_yaw_deg: f32,
    head_pitch_deg: f32,
) -> bool {
    head_eat.is_resting() && head_yaw_deg == 0.0 && head_pitch_deg == 0.0
}

/// Vanilla sheep head pose, composing `QuadrupedModel.setupAnim` (head look) with
/// the `SheepModel`/`SheepFurModel.setupAnim` overrides:
///
/// - `QuadrupedModel.setupAnim`: `head.xRot = xRot * π/180`, `head.yRot = yRot *
///   π/180` (`yRot` is the net head yaw `wrapDegrees(headRot - bodyRot)`).
/// - `SheepModel.setupAnim` (after super): `head.y += headEatPositionScale *
///   9.0 * ageScale` and `head.xRot = headEatAngleScale`, which *overrides* the pitch
///   set by the super call. Vanilla `Sheep.getHeadEatAngleScale` returns the look
///   pitch (`getXRot * π/180`) while not eating, so the head pitch is the look
///   pitch at rest and the eat curve while eating.
///
/// `BabySheepModel extends SheepModel`, so the baby head animates with `ageScale
/// = 0.5` (`LivingEntity.getAgeScale`). The base head pose has no rotation, so
/// the yaw/pitch are set (not accumulated), matching the vanilla `setupAnim`
/// assignments.
pub(in crate::entity_models) fn sheep_head_pose(
    head_pose: PartPose,
    age_scale: f32,
    head_eat: SheepHeadEatPose,
    head_yaw_deg: f32,
    head_pitch_deg: f32,
) -> PartPose {
    let x_rot = if head_eat.is_resting() {
        head_pitch_deg.to_radians()
    } else {
        head_eat.angle_scale
    };
    PartPose {
        offset: [
            head_pose.offset[0],
            head_pose.offset[1] + head_eat.position_scale * 9.0 * age_scale,
            head_pose.offset[2],
        ],
        rotation: [x_rot, head_yaw_deg.to_radians(), head_pose.rotation[2]],
    }
}

/// Vanilla `SheepModel.setupAnim`: `super.setupAnim` (the `QuadrupedModel` leg swing) then the
/// eat-grass head pose ([`sheep_head_pose`], folded with the head look). Shared by the body and fur
/// layers — both move together. The head pose is skipped while fully at rest ([`sheep_head_at_rest`]).
fn apply_sheep_anim(root: &mut ModelPart, instance: &EntityModelInstance) {
    let render_state = &instance.render_state;
    if !sheep_head_at_rest(
        render_state.kind.sheep().head_eat,
        render_state.head_yaw,
        render_state.head_pitch,
    ) {
        let head = root.child_mut("head");
        head.pose = sheep_head_pose(
            head.pose,
            render_state.age_scale,
            render_state.kind.sheep().head_eat,
            render_state.head_yaw,
            render_state.head_pitch,
        );
    }
    apply_quadruped_leg_swing(
        root,
        render_state.walk_animation_pos,
        render_state.walk_animation_speed,
    );
}

/// Mutable sheep body model, mirroring vanilla `SheepModel` (a `QuadrupedModel`). The unified tree is
/// built for `baby` with the vanilla child names; `setup_anim` runs the shared [`apply_sheep_anim`].
/// The dyed-undercoat layer renders the same tree with the wool tint.
pub(in crate::entity_models) fn sheep_model(
    baby: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let layer = if baby {
        ModelLayer::SHEEP_BABY
    } else {
        ModelLayer::SHEEP
    };
    let mut root = layer.bake();
    apply_sheep_anim(&mut root, instance);
    root
}

/// Mutable sheep fur model, mirroring vanilla `SheepFurModel`. The unified tree is the fluffy wool
/// layer for the adult; the baby reuses its body geometry (vanilla baby sheep has no separate fur
/// layer). `setup_anim` runs the same shared [`apply_sheep_anim`] so the wool moves with the body.
/// Skipped when sheared or invisible.
pub(in crate::entity_models) fn sheep_fur_model(
    baby: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let layer = if baby {
        ModelLayer::SHEEP_BABY_WOOL
    } else {
        ModelLayer::SHEEP_WOOL
    };
    let mut root = layer.bake();
    apply_sheep_anim(&mut root, instance);
    root
}
