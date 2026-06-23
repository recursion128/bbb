use super::{
    apply_head_look, model_cube as cube, ModelCubeDesc, PartPose, FOX_ORANGE, PART_POSE_ZERO,
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
pub(in crate::entity_models) const FOX_HEAD_CUBES: [ModelCubeDesc; 1] =
    [cube([-3.0, -2.0, -5.0], [8.0, 6.0, 6.0], FOX_ORANGE)];
pub(in crate::entity_models) const FOX_RIGHT_EAR_CUBES: [ModelCubeDesc; 1] =
    [cube([-3.0, -4.0, -4.0], [2.0, 2.0, 1.0], FOX_ORANGE)];
pub(in crate::entity_models) const FOX_LEFT_EAR_CUBES: [ModelCubeDesc; 1] =
    [cube([3.0, -4.0, -4.0], [2.0, 2.0, 1.0], FOX_ORANGE)];
pub(in crate::entity_models) const FOX_NOSE_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.0, 2.01, -8.0], [4.0, 2.0, 3.0], FOX_ORANGE)];

// `body`: the 6×11×6 trunk (pitched onto its belly).
pub(in crate::entity_models) const FOX_BODY_CUBES: [ModelCubeDesc; 1] =
    [cube([-3.0, 3.999, -3.5], [6.0, 11.0, 6.0], FOX_ORANGE)];

// `tail`: the 4×9×5 brush.
pub(in crate::entity_models) const FOX_TAIL_CUBES: [ModelCubeDesc; 1] =
    [cube([2.0, 0.0, -1.0], [4.0, 9.0, 5.0], FOX_ORANGE)];

// The shared leg box (all four reuse it), inflated by the vanilla `CubeDeformation(0.001)` fudge
// (min -= 0.001, size += 0.002) and built off-center at `+2` X.
pub(in crate::entity_models) const FOX_LEG_CUBES: [ModelCubeDesc; 1] = [cube(
    [1.999, 0.499, -1.001],
    [2.002, 6.002, 2.002],
    FOX_ORANGE,
)];

/// Vanilla `AdultFoxModel.createBodyLayer` rest-pose hierarchy (`addOrReplaceChild` order): `head`
/// (with ears + snout), `body` (pitched `π/2`, with the tail), then the right-hind / left-hind /
/// right-front / left-front legs. Ten cubes.
/// `head` part pose: `PartPose.offset(-1, 16.5, -3)`.
pub(in crate::entity_models) const FOX_HEAD_POSE: PartPose = PartPose {
    offset: [-1.0, 16.5, -3.0],
    rotation: [0.0, 0.0, 0.0],
};
/// The two ears and the snout all sit at the `PartPose.ZERO` head origin.
pub(in crate::entity_models) const FOX_HEAD_CHILD_POSE: PartPose = PART_POSE_ZERO;
/// `body` part pose: `PartPose.offsetAndRotation(0, 16, -6, π/2, 0, 0)`.
pub(in crate::entity_models) const FOX_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 16.0, -6.0],
    rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
};
/// `tail` part pose: `PartPose.offsetAndRotation(-4, 15, -1, -0.05235988, 0, 0)`.
pub(in crate::entity_models) const FOX_TAIL_POSE: PartPose = PartPose {
    offset: [-4.0, 15.0, -1.0],
    rotation: [-0.05235988, 0.0, 0.0],
};
/// `right_hind_leg` part pose: `PartPose.offset(-5, 17.5, 7)`.
pub(in crate::entity_models) const FOX_RIGHT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [-5.0, 17.5, 7.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_hind_leg` part pose: `PartPose.offset(-1, 17.5, 7)`.
pub(in crate::entity_models) const FOX_LEFT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [-1.0, 17.5, 7.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_front_leg` part pose: `PartPose.offset(-5, 17.5, 0)`.
pub(in crate::entity_models) const FOX_RIGHT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [-5.0, 17.5, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_front_leg` part pose: `PartPose.offset(-1, 17.5, 0)`.
pub(in crate::entity_models) const FOX_LEFT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [-1.0, 17.5, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Builds the adult fox's six named root parts under a synthetic root: the cube-bearing `head`
/// (parenting `right_ear`/`left_ear`/`nose`), the pitched `body` (parenting the `tail`), and the four
/// legs, in the vanilla `addOrReplaceChild` order.
fn fox_root() -> ModelPart {
    let head = ModelPart::colored(
        FOX_HEAD_POSE,
        &FOX_HEAD_CUBES,
        vec![
            ModelPart::leaf_colored(FOX_HEAD_CHILD_POSE, &FOX_RIGHT_EAR_CUBES),
            ModelPart::leaf_colored(FOX_HEAD_CHILD_POSE, &FOX_LEFT_EAR_CUBES),
            ModelPart::leaf_colored(FOX_HEAD_CHILD_POSE, &FOX_NOSE_CUBES),
        ],
    );
    let body = ModelPart::colored(
        FOX_BODY_POSE,
        &FOX_BODY_CUBES,
        vec![ModelPart::leaf_colored(FOX_TAIL_POSE, &FOX_TAIL_CUBES)],
    );
    ModelPart::new(
        PART_POSE_ZERO,
        Vec::new(),
        vec![
            ("head", head),
            ("body", body),
            (
                "right_hind_leg",
                ModelPart::leaf_colored(FOX_RIGHT_HIND_LEG_POSE, &FOX_LEG_CUBES),
            ),
            (
                "left_hind_leg",
                ModelPart::leaf_colored(FOX_LEFT_HIND_LEG_POSE, &FOX_LEG_CUBES),
            ),
            (
                "right_front_leg",
                ModelPart::leaf_colored(FOX_RIGHT_FRONT_LEG_POSE, &FOX_LEG_CUBES),
            ),
            (
                "left_front_leg",
                ModelPart::leaf_colored(FOX_LEFT_FRONT_LEG_POSE, &FOX_LEG_CUBES),
            ),
        ],
    )
}

// Vanilla `BabyFoxModel.createBodyLayer` (atlas 32×32). Flatter layout than the adult — the head bakes
// the ears and snout as cubes (no child parts), the body has no `π/2` pitch, and the root child order is
// head / four legs / body (the body still parenting the tail).

// `head` cubes: the 6×5×5 skull, the 2×2×2 snout, and the two 2×2×1 ears.
pub(in crate::entity_models) const BABY_FOX_HEAD_CUBES: [ModelCubeDesc; 4] = [
    cube([-3.0, -2.125, -5.125], [6.0, 5.0, 5.0], FOX_ORANGE),
    cube([-1.0, 0.875, -7.125], [2.0, 2.0, 2.0], FOX_ORANGE),
    cube([-3.0, -4.125, -4.125], [2.0, 2.0, 1.0], FOX_ORANGE),
    cube([1.0, -4.125, -4.125], [2.0, 2.0, 1.0], FOX_ORANGE),
];

// The shared 2×2×2 baby leg box.
pub(in crate::entity_models) const BABY_FOX_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.0, 0.0, -1.0], [2.0, 2.0, 2.0], FOX_ORANGE)];

// `body`: the 5×4×6 trunk.
pub(in crate::entity_models) const BABY_FOX_BODY_CUBES: [ModelCubeDesc; 1] =
    [cube([-2.5, -2.0, -3.0], [5.0, 4.0, 6.0], FOX_ORANGE)];

// `tail`: the 3×3×6 brush.
pub(in crate::entity_models) const BABY_FOX_TAIL_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.5, -1.48, -1.0], [3.0, 3.0, 6.0], FOX_ORANGE)];

/// Vanilla `BabyFoxModel.createBodyLayer` rest-pose hierarchy (`addOrReplaceChild` order): `head`
/// (ears + snout baked in), the right-hind / left-hind / right-front / left-front legs, then the
/// `body` (with the tail). Ten cubes.
/// Baby `head` part pose: `PartPose.offset(0, 18.125, 0.125)`.
pub(in crate::entity_models) const BABY_FOX_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 18.125, 0.125],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `right_hind_leg` part pose: `PartPose.offset(-1.5, 22, 4)`.
pub(in crate::entity_models) const BABY_FOX_RIGHT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [-1.5, 22.0, 4.0],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `left_hind_leg` part pose: `PartPose.offset(1.5, 22, 4)`.
pub(in crate::entity_models) const BABY_FOX_LEFT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [1.5, 22.0, 4.0],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `right_front_leg` part pose: `PartPose.offset(-1.5, 22, 0)`.
pub(in crate::entity_models) const BABY_FOX_RIGHT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [-1.5, 22.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `left_front_leg` part pose: `PartPose.offset(1.5, 22, 0)`.
pub(in crate::entity_models) const BABY_FOX_LEFT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [1.5, 22.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `body` part pose: `PartPose.offset(0, 20, 2)`.
pub(in crate::entity_models) const BABY_FOX_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 20.0, 2.0],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `tail` part pose: `PartPose.offset(0, -0.5, 3)` (no bind pitch on the baby).
pub(in crate::entity_models) const BABY_FOX_TAIL_POSE: PartPose = PartPose {
    offset: [0.0, -0.5, 3.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Builds the baby fox's six named root parts under a synthetic root: the `head` (ears + snout baked
/// in as cubes), the four legs, then the `body` (parenting the `tail`), in the vanilla
/// `addOrReplaceChild` order.
fn baby_fox_root() -> ModelPart {
    let body = ModelPart::colored(
        BABY_FOX_BODY_POSE,
        &BABY_FOX_BODY_CUBES,
        vec![ModelPart::leaf_colored(
            BABY_FOX_TAIL_POSE,
            &BABY_FOX_TAIL_CUBES,
        )],
    );
    ModelPart::new(
        PART_POSE_ZERO,
        Vec::new(),
        vec![
            (
                "head",
                ModelPart::leaf_colored(BABY_FOX_HEAD_POSE, &BABY_FOX_HEAD_CUBES),
            ),
            (
                "right_hind_leg",
                ModelPart::leaf_colored(BABY_FOX_RIGHT_HIND_LEG_POSE, &BABY_FOX_LEG_CUBES),
            ),
            (
                "left_hind_leg",
                ModelPart::leaf_colored(BABY_FOX_LEFT_HIND_LEG_POSE, &BABY_FOX_LEG_CUBES),
            ),
            (
                "right_front_leg",
                ModelPart::leaf_colored(BABY_FOX_RIGHT_FRONT_LEG_POSE, &BABY_FOX_LEG_CUBES),
            ),
            (
                "left_front_leg",
                ModelPart::leaf_colored(BABY_FOX_LEFT_FRONT_LEG_POSE, &BABY_FOX_LEG_CUBES),
            ),
            ("body", body),
        ],
    )
}

/// Mutable fox model, mirroring vanilla `AdultFoxModel` / `BabyFoxModel`. The named root parts hang off
/// a synthetic root, built from the baked colored geometry for the selected `baby` layout. Colored-only:
/// `setup_anim` runs the head look ([`apply_head_look`] on `child_mut("head")`, the head leads both
/// layouts); the walk swing, head roll, and every fox pose stay deferred.
pub(in crate::entity_models) struct FoxModel {
    root: ModelPart,
}

impl FoxModel {
    pub(in crate::entity_models) fn new(baby: bool) -> Self {
        Self {
            root: if baby { baby_fox_root() } else { fox_root() },
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
            self.root.child_mut("head"),
            instance.render_state.head_yaw,
            instance.render_state.head_pitch,
        );
    }
}
