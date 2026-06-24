use super::{
    apply_head_look, limb_swing_at_rest, model_cube as cube, ModelCubeDesc, PartPose, FELINE_TAN,
    PART_POSE_ZERO,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

// Vanilla 26.1 `AdultFelineModel.createBodyMesh(CubeDeformation.NONE)` (atlas 64×32), shared by the
// ocelot (`ModelLayers.OCELOT`, unscaled) and the cat (`ModelLayers.CAT`, the same mesh scaled 0.8 by
// `AdultCatModel.CAT_TRANSFORMER` — applied in the root transform). Eight root parts: the `head`
// (skull, nose, two ears), the pitched `body`, the two tail segments, and the four legs. The base
// `AdultFelineModel.setupAnim` sets `head.xRot/yRot` from the look and, while not sitting, drops the
// lower tail to `tail2.xRot = 1.7278761` (reproduced as the standing rest pose). Everything else stays
// deferred: the walk leg swing (its own mirrored phase + amplitude-1.0 formula, distinct from the
// `QuadrupedModel` rule) and the `tail2` walk wobble, plus the `isCrouching` / `isSprinting` /
// `isSitting` / `lieDownAmount` / `relaxStateOneAmount` poses, all reading un-projected
// `FelineRenderState` fields. The cat breed / ocelot textures and the cat collar layer are deferred, so
// the colored debug path renders one tan tint. Cat/ocelot use a plain `MobRenderer`.

/// Vanilla `AdultCatModel.CAT_TRANSFORMER = MeshTransformer.scaling(0.8)`: the cat layer is the shared
/// feline mesh scaled 0.8. The ocelot layer is unscaled.
pub(in crate::entity_models) const FELINE_CAT_SCALE: f32 = 0.8;

// `head` cubes: the 5×4×5 skull, the 3×2×2 nose, and the two 1×1×2 ears.
pub(in crate::entity_models) const FELINE_HEAD_CUBES: [ModelCubeDesc; 4] = [
    cube([-2.5, -2.0, -3.0], [5.0, 4.0, 5.0], FELINE_TAN),
    cube([-1.5, -0.001, -4.0], [3.0, 2.0, 2.0], FELINE_TAN),
    cube([-2.0, -3.0, 0.0], [1.0, 1.0, 2.0], FELINE_TAN),
    cube([1.0, -3.0, 0.0], [1.0, 1.0, 2.0], FELINE_TAN),
];

// `body`: the 4×16×6 trunk (pitched onto its belly).
pub(in crate::entity_models) const FELINE_BODY_CUBES: [ModelCubeDesc; 1] =
    [cube([-2.0, 3.0, -8.0], [4.0, 16.0, 6.0], FELINE_TAN)];

// `tail1`: the upper 1×8×1 tail segment.
pub(in crate::entity_models) const FELINE_TAIL1_CUBES: [ModelCubeDesc; 1] =
    [cube([-0.5, 0.0, 0.0], [1.0, 8.0, 1.0], FELINE_TAN)];

// `tail2`: the lower 1×8×1 tail segment, deflated by the vanilla `CubeDeformation(-0.02)` (min += 0.02,
// size -= 0.04).
pub(in crate::entity_models) const FELINE_TAIL2_CUBES: [ModelCubeDesc; 1] =
    [cube([-0.48, 0.02, 0.02], [0.96, 7.96, 0.96], FELINE_TAN)];

// The two 2×6×2 hind legs (shared box), and the two 2×10×2 front legs (shared box).
pub(in crate::entity_models) const FELINE_HIND_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.0, 0.0, 1.0], [2.0, 6.0, 2.0], FELINE_TAN)];
pub(in crate::entity_models) const FELINE_FRONT_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.0, 0.0, 0.0], [2.0, 10.0, 2.0], FELINE_TAN)];

/// Vanilla `AdultFelineModel.createBodyMesh` rest-pose hierarchy (`addOrReplaceChild` order): `head`,
/// `body` (pitched `π/2`), `tail1` (pitched `0.9`), `tail2`, then the left-hind / right-hind /
/// left-front / right-front legs. Eleven cubes.
/// `head` part pose: `PartPose.offset(0, 15, -9)`.
pub(in crate::entity_models) const FELINE_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 15.0, -9.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `body` part pose: `PartPose.offsetAndRotation(0, 12, -10, π/2, 0, 0)`.
pub(in crate::entity_models) const FELINE_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 12.0, -10.0],
    rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
};
/// `tail1` part pose: `PartPose.offsetAndRotation(0, 15, 8, 0.9, 0, 0)`.
pub(in crate::entity_models) const FELINE_TAIL1_POSE: PartPose = PartPose {
    offset: [0.0, 15.0, 8.0],
    rotation: [0.9, 0.0, 0.0],
};
/// `tail2` part pose: `PartPose.offset(0, 20, 14)`.
pub(in crate::entity_models) const FELINE_TAIL2_POSE: PartPose = PartPose {
    offset: [0.0, 20.0, 14.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_hind_leg` part pose: `PartPose.offset(1.1, 18, 5)`.
pub(in crate::entity_models) const FELINE_LEFT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [1.1, 18.0, 5.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_hind_leg` part pose: `PartPose.offset(-1.1, 18, 5)`.
pub(in crate::entity_models) const FELINE_RIGHT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [-1.1, 18.0, 5.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_front_leg` part pose: `PartPose.offset(1.2, 14.1, -5)`.
pub(in crate::entity_models) const FELINE_LEFT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [1.2, 14.1, -5.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_front_leg` part pose: `PartPose.offset(-1.2, 14.1, -5)`.
pub(in crate::entity_models) const FELINE_RIGHT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [-1.2, 14.1, -5.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Builds the adult feline's eight named root parts (`head`, `body`, `tail1`, `tail2`, then the four
/// legs) under a synthetic root, in the vanilla `addOrReplaceChild` order.
fn feline_root() -> ModelPart {
    ModelPart::new(
        PART_POSE_ZERO,
        Vec::new(),
        vec![
            (
                "head",
                ModelPart::leaf_colored(FELINE_HEAD_POSE, &FELINE_HEAD_CUBES),
            ),
            (
                "body",
                ModelPart::leaf_colored(FELINE_BODY_POSE, &FELINE_BODY_CUBES),
            ),
            (
                "tail1",
                ModelPart::leaf_colored(FELINE_TAIL1_POSE, &FELINE_TAIL1_CUBES),
            ),
            (
                "tail2",
                ModelPart::leaf_colored(FELINE_TAIL2_POSE, &FELINE_TAIL2_CUBES),
            ),
            (
                "left_hind_leg",
                ModelPart::leaf_colored(FELINE_LEFT_HIND_LEG_POSE, &FELINE_HIND_LEG_CUBES),
            ),
            (
                "right_hind_leg",
                ModelPart::leaf_colored(FELINE_RIGHT_HIND_LEG_POSE, &FELINE_HIND_LEG_CUBES),
            ),
            (
                "left_front_leg",
                ModelPart::leaf_colored(FELINE_LEFT_FRONT_LEG_POSE, &FELINE_FRONT_LEG_CUBES),
            ),
            (
                "right_front_leg",
                ModelPart::leaf_colored(FELINE_RIGHT_FRONT_LEG_POSE, &FELINE_FRONT_LEG_CUBES),
            ),
        ],
    )
}

// Vanilla 26.1 `BabyFelineModel.createBodyMesh` (atlas 32×32), shared unscaled by the baby cat
// (`ModelLayers.CAT_BABY`) and the baby ocelot (`ModelLayers.OCELOT_BABY`) — neither gets the adult
// cat's 0.8 transform. A flatter, all-upright layout: no pitched body, the head carries its ears and
// nose, and `tail2` is an empty pivot (no cube). The base `BabyFelineModel.setupAnim` still sets
// `head.xRot/yRot` from the look and `tail2.xRot = 1.7278761`, but the latter is invisible here (the
// cubeless lower tail has no geometry), so the baby's only reproduced pose is the head look.

// Baby `head` cubes (offset 0,20,-3.125): the 5×4×4 skull, two 1×1×2 ears, and a 3×2×1 nose.
pub(in crate::entity_models) const BABY_FELINE_HEAD_CUBES: [ModelCubeDesc; 4] = [
    cube([-2.5, -3.0, -2.875], [5.0, 4.0, 4.0], FELINE_TAN),
    cube([-2.0, -4.0, -0.875], [1.0, 1.0, 2.0], FELINE_TAN),
    cube([1.0, -4.0, -0.875], [1.0, 1.0, 2.0], FELINE_TAN),
    cube([-1.5, -1.0, -3.875], [3.0, 2.0, 1.0], FELINE_TAN),
];

// Baby `body`: the 4×3×7 trunk (upright, not pitched).
pub(in crate::entity_models) const BABY_FELINE_BODY_CUBES: [ModelCubeDesc; 1] =
    [cube([-2.0, -1.5, -3.5], [4.0, 3.0, 7.0], FELINE_TAN)];

// Baby legs: one shared 1×2×2 box for all four.
pub(in crate::entity_models) const BABY_FELINE_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-0.5, 0.0, -1.0], [1.0, 2.0, 2.0], FELINE_TAN)];

// Baby `tail1`: the single 1×1×5 tail segment (`tail2` below it is cubeless).
pub(in crate::entity_models) const BABY_FELINE_TAIL1_CUBES: [ModelCubeDesc; 1] =
    [cube([-0.5, -0.107, 0.0849], [1.0, 1.0, 5.0], FELINE_TAN)];

/// Vanilla `BabyFelineModel.createBodyMesh` rest-pose hierarchy (`addOrReplaceChild` order): `head`,
/// the left-front / right-front / left-hind legs, `body`, `right_hind_leg`, `tail1` (pitched
/// `-0.567232`), and the cubeless `tail2`. Ten cubes.
/// Baby `head` part pose: `PartPose.offset(0, 20, -3.125)`.
pub(in crate::entity_models) const BABY_FELINE_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 20.0, -3.125],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `left_front_leg` part pose: `PartPose.offset(1, 22, -1.5)`.
pub(in crate::entity_models) const BABY_FELINE_LEFT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [1.0, 22.0, -1.5],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `right_front_leg` part pose: `PartPose.offset(-1, 22, -1.5)`.
pub(in crate::entity_models) const BABY_FELINE_RIGHT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [-1.0, 22.0, -1.5],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `left_hind_leg` part pose: `PartPose.offset(1, 22, 2.5)`.
pub(in crate::entity_models) const BABY_FELINE_LEFT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [1.0, 22.0, 2.5],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `body` part pose: `PartPose.offset(0, 20.5, 0.5)`.
pub(in crate::entity_models) const BABY_FELINE_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 20.5, 0.5],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `right_hind_leg` part pose: `PartPose.offset(-1, 22, 2.5)`.
pub(in crate::entity_models) const BABY_FELINE_RIGHT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [-1.0, 22.0, 2.5],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `tail1` part pose: `PartPose.offsetAndRotation(0, 19.107, 3.9151, -0.567232, 0, 0)`.
pub(in crate::entity_models) const BABY_FELINE_TAIL1_POSE: PartPose = PartPose {
    offset: [0.0, 19.107, 3.9151],
    rotation: [-0.567232, 0.0, 0.0],
};
/// Baby `tail2` part pose: `PartPose.ZERO` (a cubeless pivot).
pub(in crate::entity_models) const BABY_FELINE_TAIL2_POSE: PartPose = PART_POSE_ZERO;

/// Builds the baby feline's eight named root parts under a synthetic root, in the vanilla
/// `addOrReplaceChild` order: `head`, the three front/hind legs, `body`, `right_hind_leg`, `tail1`,
/// and the cubeless `tail2`.
fn baby_feline_root() -> ModelPart {
    ModelPart::new(
        PART_POSE_ZERO,
        Vec::new(),
        vec![
            (
                "head",
                ModelPart::leaf_colored(BABY_FELINE_HEAD_POSE, &BABY_FELINE_HEAD_CUBES),
            ),
            (
                "left_front_leg",
                ModelPart::leaf_colored(BABY_FELINE_LEFT_FRONT_LEG_POSE, &BABY_FELINE_LEG_CUBES),
            ),
            (
                "right_front_leg",
                ModelPart::leaf_colored(BABY_FELINE_RIGHT_FRONT_LEG_POSE, &BABY_FELINE_LEG_CUBES),
            ),
            (
                "left_hind_leg",
                ModelPart::leaf_colored(BABY_FELINE_LEFT_HIND_LEG_POSE, &BABY_FELINE_LEG_CUBES),
            ),
            (
                "body",
                ModelPart::leaf_colored(BABY_FELINE_BODY_POSE, &BABY_FELINE_BODY_CUBES),
            ),
            (
                "right_hind_leg",
                ModelPart::leaf_colored(BABY_FELINE_RIGHT_HIND_LEG_POSE, &BABY_FELINE_LEG_CUBES),
            ),
            (
                "tail1",
                ModelPart::leaf_colored(BABY_FELINE_TAIL1_POSE, &BABY_FELINE_TAIL1_CUBES),
            ),
            (
                "tail2",
                ModelPart::leaf_colored(BABY_FELINE_TAIL2_POSE, &[]),
            ),
        ],
    )
}

/// Vanilla `AdultFelineModel.setupAnim` standing tail droop: while not sitting it sets
/// `tail2.xRot = 1.7278761` (`= π·0.55`), the base the walk wobble adds onto. With the wobble deferred
/// (walk speed dependent), this is the resting `tail2` pitch — a real change from the `0` bind rotation.
const FELINE_TAIL2_REST_X_ROT: f32 = 1.7278761;

/// Mutable feline model, mirroring vanilla `AdultFelineModel` / `BabyFelineModel` (ocelot and cat
/// share each). The named root parts hang off a synthetic root, built from the baked colored geometry;
/// the adult cat's 0.8 scale lives in the root transform ([`FELINE_CAT_SCALE`], applied by the
/// runtime — the babies are unscaled). Colored-only: `setup_anim` runs the head look
/// ([`apply_head_look`] on `child_mut("head")`) and, for the adult, drops the lower tail to its
/// standing rest pitch via `child_mut("tail2")`; the walk swing/wobble and every feline pose stay
/// deferred.
pub(in crate::entity_models) struct FelineModel {
    root: ModelPart,
    baby: bool,
}

impl FelineModel {
    pub(in crate::entity_models) fn new(baby: bool) -> Self {
        Self {
            root: if baby {
                baby_feline_root()
            } else {
                feline_root()
            },
            baby,
        }
    }
}

/// Vanilla `AdultFelineModel`/`BabyFelineModel.setupAnim` walk leg swing: each leg's `xRot = cos(pos·
/// 0.6662 [+ π]) · 1.0 · speed`. The feline uses the MIRROR of the `QuadrupedModel` diagonal at the
/// shorter `1.0` amplitude (vs the `1.4` rule) — the left-hind & right-front legs swing in phase and the
/// right-hind & left-front a half-cycle out (the opposite of the standard right-hind/left-front pairing),
/// keyed by leg NAME. The base leg pose carries no `xRot`, so it is set (not accumulated). A no-op while
/// at rest. The `tail2` walk wobble that vanilla adds on top stays deferred.
fn apply_feline_leg_swing(
    root: &mut ModelPart,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) {
    if limb_swing_at_rest(walk_animation_speed) {
        return;
    }
    let phase = walk_animation_pos * 0.6662;
    for (name, phase_offset) in [
        ("left_hind_leg", 0.0),
        ("right_front_leg", 0.0),
        ("right_hind_leg", std::f32::consts::PI),
        ("left_front_leg", std::f32::consts::PI),
    ] {
        root.child_mut(name).pose.rotation[0] = (phase + phase_offset).cos() * walk_animation_speed;
    }
}

impl EntityModel for FelineModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let render_state = &instance.render_state;
        apply_head_look(
            self.root.child_mut("head"),
            render_state.head_yaw,
            render_state.head_pitch,
        );
        // Vanilla's not-sitting branch sets `tail2.xRot = 1.7278761` unconditionally (the walk wobble
        // adds onto it); with the wobble deferred this is the resting lower-tail droop. The baby's
        // `tail2` is cubeless, so vanilla's identical assignment there is invisible — we skip it.
        if !self.baby {
            self.root.child_mut("tail2").pose.rotation[0] = FELINE_TAIL2_REST_X_ROT;
        }
        // The four legs sweep with the gait (the sitting / crouching / sprinting branches that alter it
        // are deferred, so the swing applies whenever the feline is moving).
        apply_feline_leg_swing(
            &mut self.root,
            render_state.walk_animation_pos,
            render_state.walk_animation_speed,
        );
    }
}
