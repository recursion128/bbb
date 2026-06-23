use super::super::keyframe::{
    degree_vec, keyframe, keyframe_animated_pose, keyframe_walk_sample, pos_vec,
    sample_bone_offsets, AnimationChannel, AnimationDefinition, AnimationTarget, BoneAnimation,
    Keyframe, KeyframeInterpolation,
};
use super::{
    model_cube as cube, ModelCubeDesc, PartPose, ARMADILLO_SHELL, ARMADILLO_SKIN, PART_POSE_ZERO,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

// Vanilla 26.1 `AdultArmadilloModel`/`BabyArmadilloModel.createBodyLayer` (atlas 64×64). The mesh
// root parents the body and the four legs directly (no wrapping bone); the body parents the tail
// and the head, and the head parents the head cube and the two ear planes. The armadillo is one of
// the `AgeableMobRenderer` two-model entities: `state.isBaby` (the synced `AgeableMob.DATA_BABY_ID`
// flag) selects the baby body layer, which has its own smaller geometry and a different ear/tail
// topology. The `isHidingInShell` visibility swap is now projected (see
// `adult_armadillo_rolled_root` / `baby_armadillo_rolled_root`): the synced
// `Armadillo.ArmadilloState.SCARED` shows the shell-ball `cube` and hides the body cubes, tail,
// and hind legs. While not hiding, the clamped head look ([`armadillo_clamped_head_look`]) is
// reproduced on the body-nested head pivot, and the `applyWalk` leg sway rocks the body, tail, four
// legs, and head as the armadillo moves (the head walk roll ADDS onto the look). Both the adult
// ([`ARMADILLO_WALK`]) and the baby ([`ARMADILLO_BABY_WALK`], the same bones at slightly different
// timestamps) walks are reproduced. The roll-out / roll-up / peek keyframe transition animations
// (ROLLING/UNROLLING, gated on the un-synced `inStateTicks`) stay deferred. The texture-backed path
// is deferred.

// ----- Adult -----

// `body` (offset (0, 21, 4)): a `CubeDeformation(0.3)` armor shell wrapping the bare 8×8×12 box.
pub(in crate::entity_models) const ADULT_ARMADILLO_BODY_CUBES: [ModelCubeDesc; 2] = [
    cube([-4.3, -7.3, -10.3], [8.6, 8.6, 12.6], ARMADILLO_SHELL),
    cube([-4.0, -7.0, -10.0], [8.0, 8.0, 12.0], ARMADILLO_SHELL),
];

// `tail`: a 1×6×1 plume pitched down by `0.5061` rad.
pub(in crate::entity_models) const ADULT_ARMADILLO_TAIL_CUBES: [ModelCubeDesc; 1] = [cube(
    [-0.5, -0.0865, 0.0933],
    [1.0, 6.0, 1.0],
    ARMADILLO_SKIN,
)];

// `head_cube`: the 3×5×2 snout, pitched up by `-0.3927` rad.
pub(in crate::entity_models) const ADULT_ARMADILLO_HEAD_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.5, -1.0, -1.0], [3.0, 5.0, 2.0], ARMADILLO_SKIN)];

// The two 2×5×0 ear planes (`texOffs(43,10)` / `texOffs(47,10)`).
pub(in crate::entity_models) const ADULT_ARMADILLO_RIGHT_EAR_CUBES: [ModelCubeDesc; 1] =
    [cube([-2.0, -3.0, 0.0], [2.0, 5.0, 0.0], ARMADILLO_SKIN)];
pub(in crate::entity_models) const ADULT_ARMADILLO_LEFT_EAR_CUBES: [ModelCubeDesc; 1] =
    [cube([0.0, -3.0, 0.0], [2.0, 5.0, 0.0], ARMADILLO_SKIN)];

// The shared 2×3×2 leg box (all four legs reuse it, differing only in pivot).
pub(in crate::entity_models) const ADULT_ARMADILLO_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.0, 0.0, -1.0], [2.0, 3.0, 2.0], ARMADILLO_SHELL)];

// Adult shell ball `cube` (root child at (0, 24, 0)): a plain 10×10×10 box, no deformation.
pub(in crate::entity_models) const ADULT_ARMADILLO_BALL_CUBES: [ModelCubeDesc; 1] = [cube(
    [-5.0, -10.0, -6.0],
    [10.0, 10.0, 10.0],
    ARMADILLO_SHELL,
)];

/// Vanilla `AdultArmadilloModel.createBodyLayer` rest-pose part poses. The root parents the `body`
/// and the four legs directly; the `body` parents the `tail` and the cubeless `head` pivot; the
/// `head` parents the head cube and the two ear pivots.
/// `body` part pose: `PartPose.offset(0, 21, 4)`.
pub(in crate::entity_models) const ADULT_ARMADILLO_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 21.0, 4.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `tail` part pose: `PartPose.offsetAndRotation(0, -3, 1, 0.5061, 0, 0)`.
pub(in crate::entity_models) const ADULT_ARMADILLO_TAIL_POSE: PartPose = PartPose {
    offset: [0.0, -3.0, 1.0],
    rotation: [0.5061, 0.0, 0.0],
};
/// `head` cubeless-pivot part pose: `PartPose.offset(0, -2, -11)`.
pub(in crate::entity_models) const ADULT_ARMADILLO_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, -2.0, -11.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `head_cube` part pose: `PartPose.offsetAndRotation(0, 0, 0, -0.3927, 0, 0)`.
pub(in crate::entity_models) const ADULT_ARMADILLO_HEAD_CUBE_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, 0.0],
    rotation: [-0.3927, 0.0, 0.0],
};
/// `right_ear` cubeless-pivot part pose: `PartPose.offset(-1, -1, 0)`.
pub(in crate::entity_models) const ADULT_ARMADILLO_RIGHT_EAR_POSE: PartPose = PartPose {
    offset: [-1.0, -1.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_ear_cube` part pose: `PartPose.offsetAndRotation(-0.5, 0, -0.6, 0.1886, -0.3864, -0.0718)`.
pub(in crate::entity_models) const ADULT_ARMADILLO_RIGHT_EAR_CUBE_POSE: PartPose = PartPose {
    offset: [-0.5, 0.0, -0.6],
    rotation: [0.1886, -0.3864, -0.0718],
};
/// `left_ear` cubeless-pivot part pose: `PartPose.offset(1, -2, 0)`.
pub(in crate::entity_models) const ADULT_ARMADILLO_LEFT_EAR_POSE: PartPose = PartPose {
    offset: [1.0, -2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_ear_cube` part pose: `PartPose.offsetAndRotation(0.5, 1, -0.6, 0.1886, 0.3864, 0.0718)`.
pub(in crate::entity_models) const ADULT_ARMADILLO_LEFT_EAR_CUBE_POSE: PartPose = PartPose {
    offset: [0.5, 1.0, -0.6],
    rotation: [0.1886, 0.3864, 0.0718],
};
/// `right_hind_leg` part pose: `PartPose.offset(-2, 21, 4)`.
pub(in crate::entity_models) const ADULT_ARMADILLO_RIGHT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [-2.0, 21.0, 4.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_hind_leg` part pose: `PartPose.offset(2, 21, 4)`.
pub(in crate::entity_models) const ADULT_ARMADILLO_LEFT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [2.0, 21.0, 4.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_front_leg` part pose: `PartPose.offset(-2, 21, -4)`.
pub(in crate::entity_models) const ADULT_ARMADILLO_RIGHT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [-2.0, 21.0, -4.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_front_leg` part pose: `PartPose.offset(2, 21, -4)`.
pub(in crate::entity_models) const ADULT_ARMADILLO_LEFT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [2.0, 21.0, -4.0],
    rotation: [0.0, 0.0, 0.0],
};
/// Adult shell-ball `cube` part pose: `PartPose.offset(0, 24, 0)`.
pub(in crate::entity_models) const ADULT_ARMADILLO_BALL_POSE: PartPose = PartPose {
    offset: [0.0, 24.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Builds the adult armadillo's `head` cubeless pivot, parenting the pitched head cube and the two
/// ear pivots (each carrying its rotated ear plane). Reused by the rest and rolled-up trees.
fn adult_armadillo_head() -> ModelPart {
    ModelPart::new(
        ADULT_ARMADILLO_HEAD_POSE,
        Vec::new(),
        vec![
            (
                "head_cube",
                ModelPart::leaf_colored(
                    ADULT_ARMADILLO_HEAD_CUBE_POSE,
                    &ADULT_ARMADILLO_HEAD_CUBES,
                ),
            ),
            (
                "right_ear",
                ModelPart::new(
                    ADULT_ARMADILLO_RIGHT_EAR_POSE,
                    Vec::new(),
                    vec![(
                        "right_ear_cube",
                        ModelPart::leaf_colored(
                            ADULT_ARMADILLO_RIGHT_EAR_CUBE_POSE,
                            &ADULT_ARMADILLO_RIGHT_EAR_CUBES,
                        ),
                    )],
                ),
            ),
            (
                "left_ear",
                ModelPart::new(
                    ADULT_ARMADILLO_LEFT_EAR_POSE,
                    Vec::new(),
                    vec![(
                        "left_ear_cube",
                        ModelPart::leaf_colored(
                            ADULT_ARMADILLO_LEFT_EAR_CUBE_POSE,
                            &ADULT_ARMADILLO_LEFT_EAR_CUBES,
                        ),
                    )],
                ),
            ),
        ],
    )
}

/// Builds the adult armadillo's rest-pose tree: the cube-bearing `body` (parenting the `tail` and the
/// `head`) and the four legs, in vanilla `addOrReplaceChild` order. The body, tail, head, and four
/// legs are all name-addressed by `setup_anim`.
fn adult_armadillo_root() -> ModelPart {
    let body = ModelPart::colored_named(
        ADULT_ARMADILLO_BODY_POSE,
        &ADULT_ARMADILLO_BODY_CUBES,
        vec![
            (
                "tail",
                ModelPart::leaf_colored(ADULT_ARMADILLO_TAIL_POSE, &ADULT_ARMADILLO_TAIL_CUBES),
            ),
            ("head", adult_armadillo_head()),
        ],
    );
    ModelPart::new(
        PART_POSE_ZERO,
        Vec::new(),
        vec![
            ("body", body),
            (
                "right_hind_leg",
                ModelPart::leaf_colored(
                    ADULT_ARMADILLO_RIGHT_HIND_LEG_POSE,
                    &ADULT_ARMADILLO_LEG_CUBES,
                ),
            ),
            (
                "left_hind_leg",
                ModelPart::leaf_colored(
                    ADULT_ARMADILLO_LEFT_HIND_LEG_POSE,
                    &ADULT_ARMADILLO_LEG_CUBES,
                ),
            ),
            (
                "right_front_leg",
                ModelPart::leaf_colored(
                    ADULT_ARMADILLO_RIGHT_FRONT_LEG_POSE,
                    &ADULT_ARMADILLO_LEG_CUBES,
                ),
            ),
            (
                "left_front_leg",
                ModelPart::leaf_colored(
                    ADULT_ARMADILLO_LEFT_FRONT_LEG_POSE,
                    &ADULT_ARMADILLO_LEG_CUBES,
                ),
            ),
        ],
    )
}

/// Builds the adult armadillo's rolled-up (`isHidingInShell`) tree: the body cubes (`skipDraw`),
/// the tail, and both HIND legs hide; the cubeless `body` pivot keeps just its `head` child, the two
/// FRONT legs stay, and the 10×10×10 shell-ball `cube` shows → six cubes. (Steady SCARED state only;
/// the ROLLING/UNROLLING keyframe scrunch, gated on the un-synced `inStateTicks`, stays deferred.)
fn adult_armadillo_rolled_root() -> ModelPart {
    ModelPart::new(
        PART_POSE_ZERO,
        Vec::new(),
        vec![
            (
                "body",
                ModelPart::new(
                    ADULT_ARMADILLO_BODY_POSE,
                    Vec::new(),
                    vec![("head", adult_armadillo_head())],
                ),
            ),
            (
                "right_front_leg",
                ModelPart::leaf_colored(
                    ADULT_ARMADILLO_RIGHT_FRONT_LEG_POSE,
                    &ADULT_ARMADILLO_LEG_CUBES,
                ),
            ),
            (
                "left_front_leg",
                ModelPart::leaf_colored(
                    ADULT_ARMADILLO_LEFT_FRONT_LEG_POSE,
                    &ADULT_ARMADILLO_LEG_CUBES,
                ),
            ),
            (
                "cube",
                ModelPart::leaf_colored(ADULT_ARMADILLO_BALL_POSE, &ADULT_ARMADILLO_BALL_CUBES),
            ),
        ],
    )
}

// ----- Baby -----

// `body` (offset (0, 20, 0.5)): a `CubeDeformation(0.3)` armor shell over the bare 5×4×6 box.
pub(in crate::entity_models) const BABY_ARMADILLO_BODY_CUBES: [ModelCubeDesc; 2] = [
    cube([-2.8, -2.3, -3.8], [5.6, 4.6, 7.6], ARMADILLO_SHELL),
    cube([-2.5, -2.0, -3.0], [5.0, 4.0, 6.0], ARMADILLO_SHELL),
];

// `tail` cube (vanilla names it `right_ear_cube`): a 1×1×4 stub pitched by `-1.0472` rad.
pub(in crate::entity_models) const BABY_ARMADILLO_TAIL_CUBES: [ModelCubeDesc; 1] =
    [cube([-0.5, -0.5, -2.0], [1.0, 1.0, 4.0], ARMADILLO_SKIN)];

// `head_cube`: the 2×2×4 snout, pitched up by `0.7417649` rad.
pub(in crate::entity_models) const BABY_ARMADILLO_HEAD_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.0, -2.0, -4.0], [2.0, 2.0, 4.0], ARMADILLO_SKIN)];

// The two 2×3×0 ear planes (the right one mirrored on the atlas; geometry is identical for colors).
pub(in crate::entity_models) const BABY_ARMADILLO_RIGHT_EAR_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.8, -2.0, 0.0], [2.0, 3.0, 0.0], ARMADILLO_SKIN)];
pub(in crate::entity_models) const BABY_ARMADILLO_LEFT_EAR_CUBES: [ModelCubeDesc; 1] =
    [cube([-0.2, -2.0, 0.0], [2.0, 3.0, 0.0], ARMADILLO_SKIN)];

// The shared 2×2×2 leg box.
pub(in crate::entity_models) const BABY_ARMADILLO_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.0, 0.0, -1.0], [2.0, 2.0, 2.0], ARMADILLO_SHELL)];

// Baby shell ball `cube` (root child at (0, 20.7, 0.5)): a 6×6×6 box + `CubeDeformation(0.3)` →
// min -3.3, size 6.6.
pub(in crate::entity_models) const BABY_ARMADILLO_BALL_CUBES: [ModelCubeDesc; 1] =
    [cube([-3.3, -3.3, -3.3], [6.6, 6.6, 6.6], ARMADILLO_SHELL)];

/// Vanilla `BabyArmadilloModel.createBodyLayer` rest-pose part poses: smaller geometry, the ears
/// parented to the head cube, and the front legs at swapped X origins.
/// Baby `body` part pose: `PartPose.offset(0, 20, 0.5)`.
pub(in crate::entity_models) const BABY_ARMADILLO_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 20.0, 0.5],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `tail` cubeless-pivot part pose: `PartPose.offset(0, 0, 3.4)`.
pub(in crate::entity_models) const BABY_ARMADILLO_TAIL_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, 3.4],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `tail_cube` part pose: `PartPose.offsetAndRotation(0, 1.5, 1, -1.0472, 0, 0)`.
pub(in crate::entity_models) const BABY_ARMADILLO_TAIL_CUBE_POSE: PartPose = PartPose {
    offset: [0.0, 1.5, 1.0],
    rotation: [-1.0472, 0.0, 0.0],
};
/// Baby `head` cubeless-pivot part pose: `PartPose.offset(0, 0, -3.2)`.
pub(in crate::entity_models) const BABY_ARMADILLO_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, -3.2],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `head_cube` part pose: `PartPose.offsetAndRotation(0, 0, 0, 0.7417649, 0, 0)`.
pub(in crate::entity_models) const BABY_ARMADILLO_HEAD_CUBE_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, 0.0],
    rotation: [0.7417649, 0.0, 0.0],
};
/// Baby `right_ear` part pose: `PartPose.offsetAndRotation(-1, -2, -0.3, -0.4363, -0.1134, 0.0524)`.
pub(in crate::entity_models) const BABY_ARMADILLO_RIGHT_EAR_POSE: PartPose = PartPose {
    offset: [-1.0, -2.0, -0.3],
    rotation: [-0.4363, -0.1134, 0.0524],
};
/// Baby `left_ear` part pose: `PartPose.offsetAndRotation(1, -2, -0.3, -0.4363, 0.1134, -0.0524)`.
pub(in crate::entity_models) const BABY_ARMADILLO_LEFT_EAR_POSE: PartPose = PartPose {
    offset: [1.0, -2.0, -0.3],
    rotation: [-0.4363, 0.1134, -0.0524],
};
/// Baby `right_front_leg` part pose: `PartPose.offset(-1.5, 22, 2.5)` (swapped X origin).
pub(in crate::entity_models) const BABY_ARMADILLO_RIGHT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [-1.5, 22.0, 2.5],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `left_front_leg` part pose: `PartPose.offset(1.5, 22, 2.5)` (swapped X origin).
pub(in crate::entity_models) const BABY_ARMADILLO_LEFT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [1.5, 22.0, 2.5],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `right_hind_leg` part pose: `PartPose.offset(1.5, 22, -1.5)`.
pub(in crate::entity_models) const BABY_ARMADILLO_RIGHT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [1.5, 22.0, -1.5],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `left_hind_leg` part pose: `PartPose.offset(-1.5, 22, -1.5)`.
pub(in crate::entity_models) const BABY_ARMADILLO_LEFT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [-1.5, 22.0, -1.5],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby shell-ball `cube` part pose: `PartPose.offset(0, 20.7, 0.5)`.
pub(in crate::entity_models) const BABY_ARMADILLO_BALL_POSE: PartPose = PartPose {
    offset: [0.0, 20.7, 0.5],
    rotation: [0.0, 0.0, 0.0],
};

/// Vanilla `ArmadilloModel.setupAnim` head look (only while not hiding): the pitch (`xRot`) clamps to
/// [-22.5, 25] and the yaw (`yRot`) to [-32.5, 32.5] degrees before `head.xRot/yRot` are set.
/// Returns the clamped `(yaw, pitch)` in degrees.
pub(in crate::entity_models) fn armadillo_clamped_head_look(
    head_yaw_deg: f32,
    head_pitch_deg: f32,
) -> (f32, f32) {
    (
        head_yaw_deg.clamp(-32.5, 32.5),
        head_pitch_deg.clamp(-22.5, 25.0),
    )
}

/// Builds the baby armadillo's `head` cubeless pivot, parenting the pitched head cube which itself
/// parents the two ear planes. Reused by the rest and rolled-up trees.
fn baby_armadillo_head() -> ModelPart {
    let head_cube = ModelPart::colored(
        BABY_ARMADILLO_HEAD_CUBE_POSE,
        &BABY_ARMADILLO_HEAD_CUBES,
        vec![
            ModelPart::leaf_colored(
                BABY_ARMADILLO_RIGHT_EAR_POSE,
                &BABY_ARMADILLO_RIGHT_EAR_CUBES,
            ),
            ModelPart::leaf_colored(BABY_ARMADILLO_LEFT_EAR_POSE, &BABY_ARMADILLO_LEFT_EAR_CUBES),
        ],
    );
    ModelPart::new(
        BABY_ARMADILLO_HEAD_POSE,
        Vec::new(),
        vec![("head_cube", head_cube)],
    )
}

/// Builds the baby armadillo's rest-pose tree: the cube-bearing `body` (parenting the cubeless `tail`
/// pivot → its stub cube, and the `head`) and the four legs (front legs at swapped X origins). The
/// body, tail, head, and four legs are all name-addressed by `setup_anim`.
fn baby_armadillo_root() -> ModelPart {
    let tail = ModelPart::new(
        BABY_ARMADILLO_TAIL_POSE,
        Vec::new(),
        vec![(
            "tail_cube",
            ModelPart::leaf_colored(BABY_ARMADILLO_TAIL_CUBE_POSE, &BABY_ARMADILLO_TAIL_CUBES),
        )],
    );
    let body = ModelPart::colored_named(
        BABY_ARMADILLO_BODY_POSE,
        &BABY_ARMADILLO_BODY_CUBES,
        vec![("tail", tail), ("head", baby_armadillo_head())],
    );
    ModelPart::new(
        PART_POSE_ZERO,
        Vec::new(),
        vec![
            ("body", body),
            (
                "right_hind_leg",
                ModelPart::leaf_colored(
                    BABY_ARMADILLO_RIGHT_HIND_LEG_POSE,
                    &BABY_ARMADILLO_LEG_CUBES,
                ),
            ),
            (
                "left_hind_leg",
                ModelPart::leaf_colored(
                    BABY_ARMADILLO_LEFT_HIND_LEG_POSE,
                    &BABY_ARMADILLO_LEG_CUBES,
                ),
            ),
            (
                "right_front_leg",
                ModelPart::leaf_colored(
                    BABY_ARMADILLO_RIGHT_FRONT_LEG_POSE,
                    &BABY_ARMADILLO_LEG_CUBES,
                ),
            ),
            (
                "left_front_leg",
                ModelPart::leaf_colored(
                    BABY_ARMADILLO_LEFT_FRONT_LEG_POSE,
                    &BABY_ARMADILLO_LEG_CUBES,
                ),
            ),
        ],
    )
}

/// Builds the baby armadillo's rolled-up (`isHidingInShell`) tree: same swap as the adult — the
/// cubeless `body` pivot keeps just its `head` child, the two FRONT legs stay, and the 6×6×6 shell
/// ball shows; the body cubes, tail, and both hind legs hide → six cubes.
fn baby_armadillo_rolled_root() -> ModelPart {
    ModelPart::new(
        PART_POSE_ZERO,
        Vec::new(),
        vec![
            (
                "body",
                ModelPart::new(
                    BABY_ARMADILLO_BODY_POSE,
                    Vec::new(),
                    vec![("head", baby_armadillo_head())],
                ),
            ),
            (
                "right_front_leg",
                ModelPart::leaf_colored(
                    BABY_ARMADILLO_RIGHT_FRONT_LEG_POSE,
                    &BABY_ARMADILLO_LEG_CUBES,
                ),
            ),
            (
                "left_front_leg",
                ModelPart::leaf_colored(
                    BABY_ARMADILLO_LEFT_FRONT_LEG_POSE,
                    &BABY_ARMADILLO_LEG_CUBES,
                ),
            ),
            (
                "cube",
                ModelPart::leaf_colored(BABY_ARMADILLO_BALL_POSE, &BABY_ARMADILLO_BALL_CUBES),
            ),
        ],
    )
}

// ----- `ArmadilloAnimation.ARMADILLO_WALK` (the adult walk; length 1.4583s, looping) -----
//
// `ArmadilloModel.setupAnim` samples it (while not hiding) via
// `applyWalk(walkAnimationPos, walkAnimationSpeed, 16.5, 2.5)`. The `body` channel rolls the trunk
// (a CatmullRom z-sway with a slight y-bob), the four legs swing (rotation + position), the `tail`
// rocks, and the `head` channel adds a small z-roll onto the look the head already tracks. The baby
// (`BabyArmadilloAnimation.ARMADILLO_BABY_WALK`, a different cycle and topology) stays deferred.

const LINEAR: KeyframeInterpolation = KeyframeInterpolation::Linear;
const CATMULLROM: KeyframeInterpolation = KeyframeInterpolation::CatmullRom;

const ARMADILLO_WALK_BODY_ROT: [Keyframe; 9] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.25, degree_vec(0.0, 0.0, 4.6), CATMULLROM),
    keyframe(0.2917, degree_vec(0.0, 0.0, 6.81), CATMULLROM),
    keyframe(0.5, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.7083, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.9583, degree_vec(0.0, 0.0, -4.6), CATMULLROM),
    keyframe(1.0, degree_vec(0.0, 0.0, -6.89), CATMULLROM),
    keyframe(1.25, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.4583, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const ARMADILLO_WALK_BODY_POS: [Keyframe; 7] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.25, pos_vec(0.0, -0.2, 0.0), CATMULLROM),
    keyframe(0.5, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.7083, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.9583, pos_vec(0.0, -0.2, 0.0), CATMULLROM),
    keyframe(1.25, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.4583, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const ARMADILLO_WALK_TAIL_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(-9.17, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(5.0, 0.0, 0.0), LINEAR),
    keyframe(1.2083, degree_vec(-8.24, 0.0, 0.0), LINEAR),
    keyframe(1.4583, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const ARMADILLO_WALK_RIGHT_HIND_LEG_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(-50.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(1.2917, degree_vec(-20.0, 0.0, 0.0), LINEAR),
    keyframe(1.4583, degree_vec(-50.0, 0.0, 0.0), LINEAR),
];
const ARMADILLO_WALK_RIGHT_HIND_LEG_POS: [Keyframe; 6] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(0.5, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(1.2917, pos_vec(0.0, 1.0, -0.18), LINEAR),
    keyframe(1.4583, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const ARMADILLO_WALK_LEFT_HIND_LEG_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(0.5417, degree_vec(-20.0, 0.0, 0.0), LINEAR),
    keyframe(0.7083, degree_vec(-50.0, 0.0, 0.0), LINEAR),
    keyframe(0.9583, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.2083, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.4583, degree_vec(50.0, 0.0, 0.0), LINEAR),
];
const ARMADILLO_WALK_LEFT_HIND_LEG_POS: [Keyframe; 7] = [
    keyframe(0.0, pos_vec(0.0, 0.0, -0.25), LINEAR),
    keyframe(0.25, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(0.5417, pos_vec(0.0, 1.0, -0.18), LINEAR),
    keyframe(0.7083, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.9583, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(1.2083, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.4583, pos_vec(0.0, 0.0, -0.25), LINEAR),
];
const ARMADILLO_WALK_RIGHT_FRONT_LEG_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(0.2917, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(0.5417, degree_vec(-20.0, 0.0, 0.0), LINEAR),
    keyframe(0.7083, degree_vec(-50.0, 0.0, 0.0), LINEAR),
    keyframe(0.9583, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.2083, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.4583, degree_vec(50.0, 0.0, 0.0), LINEAR),
];
const ARMADILLO_WALK_RIGHT_FRONT_LEG_POS: [Keyframe; 7] = [
    keyframe(0.0, pos_vec(0.0, 0.0, -0.25), LINEAR),
    keyframe(0.25, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(0.5417, pos_vec(0.0, 1.0, -0.18), LINEAR),
    keyframe(0.7083, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.9583, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(1.2083, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.4583, pos_vec(0.0, 0.0, -0.25), LINEAR),
];
const ARMADILLO_WALK_LEFT_FRONT_LEG_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(-50.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(1.2917, degree_vec(-20.0, 0.0, 0.0), LINEAR),
    keyframe(1.4583, degree_vec(-50.0, 0.0, 0.0), LINEAR),
];
const ARMADILLO_WALK_LEFT_FRONT_LEG_POS: [Keyframe; 6] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(0.5, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(1.2917, pos_vec(0.0, 1.0, -0.18), LINEAR),
    keyframe(1.4583, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const ARMADILLO_WALK_HEAD_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(0.0, 0.0, -2.5), LINEAR),
    keyframe(0.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.7083, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(0.0, 0.0, 2.5), LINEAR),
    keyframe(1.25, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.4583, degree_vec(0.0, 0.0, 0.0), LINEAR),
];

const fn rot(keyframes: &'static [Keyframe]) -> AnimationChannel {
    AnimationChannel {
        target: AnimationTarget::Rotation,
        keyframes,
    }
}
const fn pos(keyframes: &'static [Keyframe]) -> AnimationChannel {
    AnimationChannel {
        target: AnimationTarget::Position,
        keyframes,
    }
}

const ARMADILLO_WALK_BODY_CHANNELS: [AnimationChannel; 2] =
    [rot(&ARMADILLO_WALK_BODY_ROT), pos(&ARMADILLO_WALK_BODY_POS)];
const ARMADILLO_WALK_TAIL_CHANNELS: [AnimationChannel; 1] = [rot(&ARMADILLO_WALK_TAIL_ROT)];
const ARMADILLO_WALK_RIGHT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&ARMADILLO_WALK_RIGHT_HIND_LEG_ROT),
    pos(&ARMADILLO_WALK_RIGHT_HIND_LEG_POS),
];
const ARMADILLO_WALK_LEFT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&ARMADILLO_WALK_LEFT_HIND_LEG_ROT),
    pos(&ARMADILLO_WALK_LEFT_HIND_LEG_POS),
];
const ARMADILLO_WALK_RIGHT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&ARMADILLO_WALK_RIGHT_FRONT_LEG_ROT),
    pos(&ARMADILLO_WALK_RIGHT_FRONT_LEG_POS),
];
const ARMADILLO_WALK_LEFT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&ARMADILLO_WALK_LEFT_FRONT_LEG_ROT),
    pos(&ARMADILLO_WALK_LEFT_FRONT_LEG_POS),
];
const ARMADILLO_WALK_HEAD_CHANNELS: [AnimationChannel; 1] = [rot(&ARMADILLO_WALK_HEAD_ROT)];

const ARMADILLO_WALK_BONES: [BoneAnimation; 7] = [
    BoneAnimation {
        bone: "body",
        channels: &ARMADILLO_WALK_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "tail",
        channels: &ARMADILLO_WALK_TAIL_CHANNELS,
    },
    BoneAnimation {
        bone: "right_hind_leg",
        channels: &ARMADILLO_WALK_RIGHT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_hind_leg",
        channels: &ARMADILLO_WALK_LEFT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "right_front_leg",
        channels: &ARMADILLO_WALK_RIGHT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_front_leg",
        channels: &ARMADILLO_WALK_LEFT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &ARMADILLO_WALK_HEAD_CHANNELS,
    },
];

/// Vanilla `ArmadilloAnimation.ARMADILLO_WALK`: the looping 1.4583s adult walk cycle, sampled by
/// `ArmadilloModel.setupAnim` (while not hiding) via
/// `applyWalk(walkAnimationPos, walkAnimationSpeed, 16.5, 2.5)`. The `head` channel adds onto the
/// clamped look the head already tracks; the `body` channel uses CatmullRom interpolation.
pub(in crate::entity_models) const ARMADILLO_WALK: AnimationDefinition = AnimationDefinition {
    length_seconds: 1.4583,
    looping: true,
    bones: &ARMADILLO_WALK_BONES,
};

/// Vanilla `ArmadilloModel.applyWalk(..., 16.5F, 2.5F)` factors: `MAX_WALK_ANIMATION_SPEED` drives
/// the sample time and `WALK_ANIMATION_SCALE_FACTOR` the amplitude. The base `ArmadilloModel` passes
/// these for both the adult and the baby walk.
pub(in crate::entity_models) const ARMADILLO_WALK_SPEED_FACTOR: f32 = 16.5;
pub(in crate::entity_models) const ARMADILLO_WALK_SCALE_FACTOR: f32 = 2.5;

// ----- `BabyArmadilloAnimation.ARMADILLO_BABY_WALK` (the baby walk; length 1.4583s, looping) -----
//
// The same seven bones (body / tail / four legs / head) and value structure as the adult walk, with
// slightly different keyframe timestamps. The baby topology differs (the tail parents a stub cube,
// the head cube parents the ears), but the animated bone names line up.

const ARMADILLO_BABY_WALK_BODY_ROT: [Keyframe; 9] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.25, degree_vec(0.0, 0.0, 4.6), CATMULLROM),
    keyframe(0.3, degree_vec(0.0, 0.0, 6.81), CATMULLROM),
    keyframe(0.5, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.7, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.95, degree_vec(0.0, 0.0, -4.6), CATMULLROM),
    keyframe(1.0, degree_vec(0.0, 0.0, -6.89), CATMULLROM),
    keyframe(1.25, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.45, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const ARMADILLO_BABY_WALK_BODY_POS: [Keyframe; 7] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.25, pos_vec(0.0, -0.2, 0.0), CATMULLROM),
    keyframe(0.5, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.7, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.95, pos_vec(0.0, -0.2, 0.0), CATMULLROM),
    keyframe(1.25, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.45, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const ARMADILLO_BABY_WALK_TAIL_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(-9.17, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(5.0, 0.0, 0.0), LINEAR),
    keyframe(1.2, degree_vec(-8.24, 0.0, 0.0), LINEAR),
    keyframe(1.45, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const ARMADILLO_BABY_WALK_RIGHT_HIND_LEG_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(-50.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(1.3, degree_vec(-20.0, 0.0, 0.0), LINEAR),
    keyframe(1.45, degree_vec(-50.0, 0.0, 0.0), LINEAR),
];
const ARMADILLO_BABY_WALK_RIGHT_HIND_LEG_POS: [Keyframe; 6] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(0.5, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(1.3, pos_vec(0.0, 1.0, -0.18), LINEAR),
    keyframe(1.45, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const ARMADILLO_BABY_WALK_LEFT_HIND_LEG_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(0.55, degree_vec(-20.0, 0.0, 0.0), LINEAR),
    keyframe(0.7, degree_vec(-50.0, 0.0, 0.0), LINEAR),
    keyframe(0.95, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.2, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.45, degree_vec(50.0, 0.0, 0.0), LINEAR),
];
const ARMADILLO_BABY_WALK_LEFT_HIND_LEG_POS: [Keyframe; 7] = [
    keyframe(0.0, pos_vec(0.0, 0.0, -0.25), LINEAR),
    keyframe(0.25, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(0.55, pos_vec(0.0, 1.0, -0.18), LINEAR),
    keyframe(0.7, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.95, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(1.2, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.45, pos_vec(0.0, 0.0, -0.25), LINEAR),
];
const ARMADILLO_BABY_WALK_RIGHT_FRONT_LEG_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(0.3, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(0.55, degree_vec(-20.0, 0.0, 0.0), LINEAR),
    keyframe(0.7, degree_vec(-50.0, 0.0, 0.0), LINEAR),
    keyframe(0.95, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.2, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.45, degree_vec(50.0, 0.0, 0.0), LINEAR),
];
const ARMADILLO_BABY_WALK_RIGHT_FRONT_LEG_POS: [Keyframe; 7] = [
    keyframe(0.0, pos_vec(0.0, 0.0, -0.25), LINEAR),
    keyframe(0.25, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(0.55, pos_vec(0.0, 1.0, -0.18), LINEAR),
    keyframe(0.7, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.95, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(1.2, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.45, pos_vec(0.0, 0.0, -0.25), LINEAR),
];
const ARMADILLO_BABY_WALK_LEFT_FRONT_LEG_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(-50.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(50.0, 0.0, 0.0), LINEAR),
    keyframe(1.3, degree_vec(-20.0, 0.0, 0.0), LINEAR),
    keyframe(1.45, degree_vec(-50.0, 0.0, 0.0), LINEAR),
];
const ARMADILLO_BABY_WALK_LEFT_FRONT_LEG_POS: [Keyframe; 6] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(0.5, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, pos_vec(0.0, 0.0, -0.5), LINEAR),
    keyframe(1.3, pos_vec(0.0, 1.0, -0.18), LINEAR),
    keyframe(1.45, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const ARMADILLO_BABY_WALK_HEAD_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(0.0, 0.0, -2.5), LINEAR),
    keyframe(0.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.7, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.0, degree_vec(0.0, 0.0, 2.5), LINEAR),
    keyframe(1.25, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.45, degree_vec(0.0, 0.0, 0.0), LINEAR),
];

const ARMADILLO_BABY_WALK_BODY_CHANNELS: [AnimationChannel; 2] = [
    rot(&ARMADILLO_BABY_WALK_BODY_ROT),
    pos(&ARMADILLO_BABY_WALK_BODY_POS),
];
const ARMADILLO_BABY_WALK_TAIL_CHANNELS: [AnimationChannel; 1] =
    [rot(&ARMADILLO_BABY_WALK_TAIL_ROT)];
const ARMADILLO_BABY_WALK_RIGHT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&ARMADILLO_BABY_WALK_RIGHT_HIND_LEG_ROT),
    pos(&ARMADILLO_BABY_WALK_RIGHT_HIND_LEG_POS),
];
const ARMADILLO_BABY_WALK_LEFT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&ARMADILLO_BABY_WALK_LEFT_HIND_LEG_ROT),
    pos(&ARMADILLO_BABY_WALK_LEFT_HIND_LEG_POS),
];
const ARMADILLO_BABY_WALK_RIGHT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&ARMADILLO_BABY_WALK_RIGHT_FRONT_LEG_ROT),
    pos(&ARMADILLO_BABY_WALK_RIGHT_FRONT_LEG_POS),
];
const ARMADILLO_BABY_WALK_LEFT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&ARMADILLO_BABY_WALK_LEFT_FRONT_LEG_ROT),
    pos(&ARMADILLO_BABY_WALK_LEFT_FRONT_LEG_POS),
];
const ARMADILLO_BABY_WALK_HEAD_CHANNELS: [AnimationChannel; 1] =
    [rot(&ARMADILLO_BABY_WALK_HEAD_ROT)];

const ARMADILLO_BABY_WALK_BONES: [BoneAnimation; 7] = [
    BoneAnimation {
        bone: "body",
        channels: &ARMADILLO_BABY_WALK_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "tail",
        channels: &ARMADILLO_BABY_WALK_TAIL_CHANNELS,
    },
    BoneAnimation {
        bone: "right_hind_leg",
        channels: &ARMADILLO_BABY_WALK_RIGHT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_hind_leg",
        channels: &ARMADILLO_BABY_WALK_LEFT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "right_front_leg",
        channels: &ARMADILLO_BABY_WALK_RIGHT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_front_leg",
        channels: &ARMADILLO_BABY_WALK_LEFT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &ARMADILLO_BABY_WALK_HEAD_CHANNELS,
    },
];

/// Vanilla `BabyArmadilloAnimation.ARMADILLO_BABY_WALK`: the looping 1.4583s baby walk cycle,
/// sampled like the adult via `applyWalk(walkAnimationPos, walkAnimationSpeed, 16.5, 2.5)`. Same
/// seven bones and structure as [`ARMADILLO_WALK`], with slightly different keyframe timestamps.
pub(in crate::entity_models) const ARMADILLO_BABY_WALK: AnimationDefinition = AnimationDefinition {
    length_seconds: 1.4583,
    looping: true,
    bones: &ARMADILLO_BABY_WALK_BONES,
};

/// Mutable armadillo model, mirroring vanilla `AdultArmadilloModel` / `BabyArmadilloModel`. The
/// body (parenting the tail and head; the head parents the head cube and ears) and four legs hang
/// off a synthetic root, built from the baked adult/baby geometry selected at construction. When
/// `rolled_up` (the synced `ArmadilloState.SCARED`), the shell-ball variant tree is used and held
/// static (no head look, no walk). Colored-only: while not hiding, `setup_anim` sets the clamped
/// head look and adds the looping walk cycle onto the body, tail, head, and four legs.
pub(in crate::entity_models) struct ArmadilloModel {
    root: ModelPart,
    baby: bool,
    rolled_up: bool,
}

impl ArmadilloModel {
    pub(in crate::entity_models) fn new(baby: bool, rolled_up: bool) -> Self {
        let root = match (baby, rolled_up) {
            (false, false) => adult_armadillo_root(),
            (true, false) => baby_armadillo_root(),
            (false, true) => adult_armadillo_rolled_root(),
            (true, true) => baby_armadillo_rolled_root(),
        };
        Self {
            root,
            baby,
            rolled_up,
        }
    }
}

impl EntityModel for ArmadilloModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        // While hiding in its shell the rolled-up variant tree is rendered as-is (vanilla shows the
        // shell ball with no head look or walk), so `setup_anim` is a no-op.
        if self.rolled_up {
            return;
        }
        // Vanilla `ArmadilloModel.setupAnim` (not hiding): the clamped head look, then
        // `applyWalk(walkAnimationPos, walkAnimationSpeed, 16.5, 2.5)`. The walk rolls the body
        // (CatmullRom z-sway + y-bob, carrying the tail and head), rocks the tail, adds a head z-roll
        // ONTO the clamped look, and swings the four legs. A still armadillo samples amplitude 0,
        // collapsing to the bind pose plus the head look. The adult and baby share this topology.
        let walk: &AnimationDefinition = if self.baby {
            &ARMADILLO_BABY_WALK
        } else {
            &ARMADILLO_WALK
        };
        let (head_yaw, head_pitch) = armadillo_clamped_head_look(
            instance.render_state.head_yaw,
            instance.render_state.head_pitch,
        );
        let (seconds, scale) = keyframe_walk_sample(
            walk,
            instance.render_state.walk_animation_pos,
            instance.render_state.walk_animation_speed,
            ARMADILLO_WALK_SPEED_FACTOR,
            ARMADILLO_WALK_SCALE_FACTOR,
        );
        let animate = |part: &mut ModelPart, bone: &str| {
            let (position, rotation) = sample_bone_offsets(walk, bone, seconds, scale);
            part.pose = keyframe_animated_pose(part.pose, position, rotation);
        };

        {
            let body = self.root.child_mut("body");
            animate(body, "body");
            animate(body.child_mut("tail"), "tail");

            // The body-nested `head`: the clamped look (set) plus the walk z-roll (added).
            let head = body.child_mut("head");
            let (_, head_walk_rot) = sample_bone_offsets(walk, "head", seconds, scale);
            head.pose.rotation = [
                head_pitch.to_radians() + head_walk_rot[0],
                head_yaw.to_radians() + head_walk_rot[1],
                head.pose.rotation[2] + head_walk_rot[2],
            ];
        }
        for bone in [
            "right_hind_leg",
            "left_hind_leg",
            "right_front_leg",
            "left_front_leg",
        ] {
            animate(self.root.child_mut(bone), bone);
        }
    }
}
