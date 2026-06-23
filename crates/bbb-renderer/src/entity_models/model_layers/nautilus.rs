use super::{
    apply_head_look, bind_part as part, model_cube as cube, ModelCubeDesc, ModelPartDesc,
    NAUTILUS_BODY, NAUTILUS_SHELL,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

// Vanilla 26.1 `NautilusModel.createBodyLayer` (atlas 128×128) — the new rideable nautilus mob.
// `NautilusModel extends EntityModel`: one cubeless `root` pivot parenting the `shell` (the spiral shell,
// three boxes) and the `body` (the trunk plus its 0-thickness rear fin) which in turn parents the three
// mouth boxes. `NautilusModel.setupAnim` steers the `body` by the look — `body.yRot/xRot` set from the
// look yaw/pitch CLAMPED to ±10° — then layers the looping `NautilusAnimation.SWIMMING` keyframe via
// `applyWalk` (always on, the idle baseline `walkAnimationSpeed + 0.2`). The clamped body look is
// reproduced; the SWIMMING keyframe undulation needs the keyframe machinery + an `AnimationState`, so it
// stays deferred (the nautilus renders at this bind pose plus the clamped look). The variant textures,
// the saddle / armor / coral layers, and the distinct baby `createBabyBodyLayer` mesh are deferred, so
// the colored debug path renders a tan shell over a pale body. Nautilus uses a plain `MobRenderer`.

// `shell` cubes: the 14×10×16 dome, the 14×8×20 whorl, and a 14×8×0 rear fin plane.
const NAUTILUS_SHELL_CUBES: [ModelCubeDesc; 3] = [
    cube([-7.0, -10.0, -7.0], [14.0, 10.0, 16.0], NAUTILUS_SHELL),
    cube([-7.0, 0.0, -7.0], [14.0, 8.0, 20.0], NAUTILUS_SHELL),
    cube([-7.0, 0.0, 6.0], [14.0, 8.0, 0.0], NAUTILUS_SHELL),
];

// `body` cubes: the 10×8×14 trunk and a 10×8×0 rear fin plane.
const NAUTILUS_BODY_CUBES: [ModelCubeDesc; 2] = [
    cube([-5.0, -4.51, -3.0], [10.0, 8.0, 14.0], NAUTILUS_BODY),
    cube([-5.0, -4.51, 7.0], [10.0, 8.0, 0.0], NAUTILUS_BODY),
];

// The three mouth boxes. Upper/lower use the vanilla `CubeDeformation(-0.001)` (min += 0.001,
// size -= 0.002); the inner mouth is undeformed.
const NAUTILUS_UPPER_MOUTH_CUBES: [ModelCubeDesc; 1] = [cube(
    [-4.999, -1.999, 0.001],
    [9.998, 3.998, 3.998],
    NAUTILUS_BODY,
)];
const NAUTILUS_INNER_MOUTH_CUBES: [ModelCubeDesc; 1] =
    [cube([-3.0, -2.0, -0.5], [6.0, 4.0, 4.0], NAUTILUS_BODY)];
const NAUTILUS_LOWER_MOUTH_CUBES: [ModelCubeDesc; 1] = [cube(
    [-4.999, -1.979, 0.001],
    [9.998, 3.998, 3.998],
    NAUTILUS_BODY,
)];

// `body` children: the three mouth boxes.
const NAUTILUS_BODY_CHILDREN: [ModelPartDesc; 3] = [
    part([0.0, -2.51, 7.0], &NAUTILUS_UPPER_MOUTH_CUBES, &[]),
    part([0.0, -0.51, 7.5], &NAUTILUS_INNER_MOUTH_CUBES, &[]),
    part([0.0, 1.49, 7.0], &NAUTILUS_LOWER_MOUTH_CUBES, &[]),
];

// The cubeless `root` pivot's children: the `shell` and the `body` (with its mouths).
const NAUTILUS_ROOT_CHILDREN: [ModelPartDesc; 2] = [
    part([0.0, -13.0, 5.0], &NAUTILUS_SHELL_CUBES, &[]),
    part(
        [0.0, -8.5, 12.3],
        &NAUTILUS_BODY_CUBES,
        &NAUTILUS_BODY_CHILDREN,
    ),
];

/// Vanilla `NautilusModel.createBodyMesh` rest-pose hierarchy: the cubeless `root` pivot at
/// `offset(0, 29, -6)` parenting the `shell` (3 cubes) and the `body` (2 cubes, parenting the three
/// 1-cube mouths). Eight cubes.
pub(in crate::entity_models) const NAUTILUS_PARTS: [ModelPartDesc; 1] =
    [part([0.0, 29.0, -6.0], &[], &NAUTILUS_ROOT_CHILDREN)];

/// Vanilla `NautilusModel.applyBodyRotation` clamps the look yaw/pitch to ±10° before steering the body.
const NAUTILUS_LOOK_CLAMP_DEGREES: f32 = 10.0;

/// Mutable nautilus model, mirroring vanilla `NautilusModel`. The hierarchy hangs off a synthetic root,
/// built from the baked [`NAUTILUS_PARTS`] geometry. Colored-only: `setup_anim` steers the body by the
/// clamped look ([`apply_head_look`] on the body part, with the yaw/pitch pre-clamped to ±10°); the
/// SWIMMING keyframe undulation and every other pose stay deferred.
pub(in crate::entity_models) struct NautilusModel {
    root: ModelPart,
}

impl NautilusModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::root_from_colored_descs(&NAUTILUS_PARTS),
        }
    }
}

impl EntityModel for NautilusModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        // Vanilla `NautilusModel.applyBodyRotation`: the body (root → body) turns to the look, clamped
        // to ±10° on each axis. `apply_head_look` sets `xRot`/`yRot` from the (pre-clamped) degrees.
        let render_state = &instance.render_state;
        let body = self.root.child_at_mut(0).child_at_mut(1);
        apply_head_look(
            body,
            render_state
                .head_yaw
                .clamp(-NAUTILUS_LOOK_CLAMP_DEGREES, NAUTILUS_LOOK_CLAMP_DEGREES),
            render_state
                .head_pitch
                .clamp(-NAUTILUS_LOOK_CLAMP_DEGREES, NAUTILUS_LOOK_CLAMP_DEGREES),
        );
    }
}
