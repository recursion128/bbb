use super::{
    apply_head_look, bind_part as part, bind_part_rot as rpart, model_cube as cube, ModelCubeDesc,
    ModelPartDesc, FOX_ORANGE,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

// Vanilla 26.1 `AdultFoxModel.createBodyLayer` (atlas 48×32). `FoxModel extends EntityModel`: six root
// parts — the `head` (carrying the two ears and the snout), the pitched `body` (carrying the tail), and
// the four legs (all sharing one fudge-inflated 2×6×2 box built off-center at `+2` X). At rest (not
// sleeping / faceplanted / crouching) `FoxModel.setupAnim` sets `head.xRot/yRot` from the look; the tail,
// head roll, and legs keep their bind pose. Everything else is deferred: the walk leg swing (the standard
// `cos·1.4·speed` but keyed left/right by part rather than pivot sign, since the fox's legs are all built
// at negative pivot X, so it can't reuse the `QuadrupedModel` `x·z` helper), the `headRollAngle` head
// tilt, and the `isCrouching` / `isSleeping` / `isSitting` / `isPouncing` / `isFaceplanted` poses, all
// reading un-projected `FoxRenderState` state. The four `Fox.Variant` (red/snow) idle/sleeping textures
// and the held-item layer are deferred, so the colored debug path renders one orange tint. The pounce /
// faceplant `FoxRenderer.setupRotations` pitch is deferred too.

// `head` cubes: the 8×6×6 skull, the two 2×2×1 ears, and the 4×2×3 snout.
const FOX_HEAD_CUBES: [ModelCubeDesc; 1] = [cube([-3.0, -2.0, -5.0], [8.0, 6.0, 6.0], FOX_ORANGE)];
const FOX_RIGHT_EAR_CUBES: [ModelCubeDesc; 1] =
    [cube([-3.0, -4.0, -4.0], [2.0, 2.0, 1.0], FOX_ORANGE)];
const FOX_LEFT_EAR_CUBES: [ModelCubeDesc; 1] =
    [cube([3.0, -4.0, -4.0], [2.0, 2.0, 1.0], FOX_ORANGE)];
const FOX_NOSE_CUBES: [ModelCubeDesc; 1] = [cube([-1.0, 2.01, -8.0], [4.0, 2.0, 3.0], FOX_ORANGE)];

// `body`: the 6×11×6 trunk (pitched onto its belly).
const FOX_BODY_CUBES: [ModelCubeDesc; 1] =
    [cube([-3.0, 3.999, -3.5], [6.0, 11.0, 6.0], FOX_ORANGE)];

// `tail`: the 4×9×5 brush.
const FOX_TAIL_CUBES: [ModelCubeDesc; 1] = [cube([2.0, 0.0, -1.0], [4.0, 9.0, 5.0], FOX_ORANGE)];

// The shared leg box (all four reuse it), inflated by the vanilla `CubeDeformation(0.001)` fudge
// (min -= 0.001, size += 0.002) and built off-center at `+2` X.
const FOX_LEG_CUBES: [ModelCubeDesc; 1] = [cube(
    [1.999, 0.499, -1.001],
    [2.002, 6.002, 2.002],
    FOX_ORANGE,
)];

// `head` children: the two ears and the snout, all at the `PartPose.ZERO` head origin.
const FOX_HEAD_CHILDREN: [ModelPartDesc; 3] = [
    part([0.0, 0.0, 0.0], &FOX_RIGHT_EAR_CUBES, &[]),
    part([0.0, 0.0, 0.0], &FOX_LEFT_EAR_CUBES, &[]),
    part([0.0, 0.0, 0.0], &FOX_NOSE_CUBES, &[]),
];

// `body` child: the tail, pitched back `-0.05235988`.
const FOX_BODY_CHILDREN: [ModelPartDesc; 1] = [rpart(
    [-4.0, 15.0, -1.0],
    [-0.05235988, 0.0, 0.0],
    &FOX_TAIL_CUBES,
    &[],
)];

/// Vanilla `AdultFoxModel.createBodyLayer` rest-pose hierarchy (`addOrReplaceChild` order): `head` (0,
/// with ears + snout), `body` (1, pitched `π/2`, with the tail), then the right-hind / left-hind /
/// right-front / left-front legs (2..=5). Ten cubes.
pub(in crate::entity_models) const FOX_PARTS: [ModelPartDesc; 6] = [
    part([-1.0, 16.5, -3.0], &FOX_HEAD_CUBES, &FOX_HEAD_CHILDREN),
    rpart(
        [0.0, 16.0, -6.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        &FOX_BODY_CUBES,
        &FOX_BODY_CHILDREN,
    ),
    part([-5.0, 17.5, 7.0], &FOX_LEG_CUBES, &[]),
    part([-1.0, 17.5, 7.0], &FOX_LEG_CUBES, &[]),
    part([-5.0, 17.5, 0.0], &FOX_LEG_CUBES, &[]),
    part([-1.0, 17.5, 0.0], &FOX_LEG_CUBES, &[]),
];

// Vanilla `BabyFoxModel.createBodyLayer` (atlas 32×32). Flatter layout than the adult — the head bakes
// the ears and snout as cubes (no child parts), the body has no `π/2` pitch, and the root child order is
// head / four legs / body (the body still parenting the tail).

// `head` cubes: the 6×5×5 skull, the 2×2×2 snout, and the two 2×2×1 ears.
const BABY_FOX_HEAD_CUBES: [ModelCubeDesc; 4] = [
    cube([-3.0, -2.125, -5.125], [6.0, 5.0, 5.0], FOX_ORANGE),
    cube([-1.0, 0.875, -7.125], [2.0, 2.0, 2.0], FOX_ORANGE),
    cube([-3.0, -4.125, -4.125], [2.0, 2.0, 1.0], FOX_ORANGE),
    cube([1.0, -4.125, -4.125], [2.0, 2.0, 1.0], FOX_ORANGE),
];

// The shared 2×2×2 baby leg box.
const BABY_FOX_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.0, 0.0, -1.0], [2.0, 2.0, 2.0], FOX_ORANGE)];

// `body`: the 5×4×6 trunk.
const BABY_FOX_BODY_CUBES: [ModelCubeDesc; 1] =
    [cube([-2.5, -2.0, -3.0], [5.0, 4.0, 6.0], FOX_ORANGE)];

// `tail`: the 3×3×6 brush.
const BABY_FOX_TAIL_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.5, -1.48, -1.0], [3.0, 3.0, 6.0], FOX_ORANGE)];

// `body` child: the tail (no bind pitch on the baby).
const BABY_FOX_BODY_CHILDREN: [ModelPartDesc; 1] =
    [part([0.0, -0.5, 3.0], &BABY_FOX_TAIL_CUBES, &[])];

/// Vanilla `BabyFoxModel.createBodyLayer` rest-pose hierarchy (`addOrReplaceChild` order): `head` (0,
/// ears + snout baked in), the right-hind / left-hind / right-front / left-front legs (1..=4), then the
/// `body` (5, with the tail). Ten cubes.
pub(in crate::entity_models) const BABY_FOX_PARTS: [ModelPartDesc; 6] = [
    part([0.0, 18.125, 0.125], &BABY_FOX_HEAD_CUBES, &[]),
    part([-1.5, 22.0, 4.0], &BABY_FOX_LEG_CUBES, &[]),
    part([1.5, 22.0, 4.0], &BABY_FOX_LEG_CUBES, &[]),
    part([-1.5, 22.0, 0.0], &BABY_FOX_LEG_CUBES, &[]),
    part([1.5, 22.0, 0.0], &BABY_FOX_LEG_CUBES, &[]),
    part(
        [0.0, 20.0, 2.0],
        &BABY_FOX_BODY_CUBES,
        &BABY_FOX_BODY_CHILDREN,
    ),
];

/// The `head` is the first root part in both the adult and baby layouts; `FoxModel.setupAnim` sets its
/// `xRot/yRot` from the look.
const FOX_HEAD_PART_INDEX: usize = 0;

/// Selects the baked fox part tree for `baby`.
fn fox_parts(baby: bool) -> &'static [ModelPartDesc] {
    if baby {
        &BABY_FOX_PARTS
    } else {
        &FOX_PARTS
    }
}

/// Mutable fox model, mirroring vanilla `AdultFoxModel` / `BabyFoxModel`. The root parts hang off a
/// synthetic root, built from the baked [`FOX_PARTS`] / [`BABY_FOX_PARTS`] geometry for the selected
/// `baby` layout. Colored-only: `setup_anim` runs the head look ([`apply_head_look`], the head is part 0
/// in both layouts); the walk swing, head roll, and every fox pose stay deferred.
pub(in crate::entity_models) struct FoxModel {
    root: ModelPart,
}

impl FoxModel {
    pub(in crate::entity_models) fn new(baby: bool) -> Self {
        Self {
            root: ModelPart::root_from_colored_descs(fox_parts(baby)),
        }
    }
}

impl EntityModel for FoxModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        // Vanilla `FoxModel.setupAnim` sets `head.xRot/yRot` from the look while the fox is not sleeping
        // / faceplanted / crouching — none of which bbb projects, so the look applies every frame.
        apply_head_look(
            self.root.child_at_mut(FOX_HEAD_PART_INDEX),
            instance.render_state.head_yaw,
            instance.render_state.head_pitch,
        );
    }
}
