use super::{apply_head_look, limb_swing_at_rest, PartPose, FELINE_TAN, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_CAT: &str = "minecraft:cat#main";
pub(in crate::entity_models) const MODEL_LAYER_CAT_BABY: &str = "minecraft:cat_baby#main";
pub(in crate::entity_models) const MODEL_LAYER_CAT_COLLAR: &str = "minecraft:cat#collar";
pub(in crate::entity_models) const MODEL_LAYER_CAT_BABY_COLLAR: &str = "minecraft:cat_baby#collar";
pub(in crate::entity_models) const MODEL_LAYER_OCELOT: &str = "minecraft:ocelot#main";
pub(in crate::entity_models) const MODEL_LAYER_OCELOT_BABY: &str = "minecraft:ocelot_baby#main";

// Vanilla 26.1 `AdultFelineModel.createBodyMesh(CubeDeformation.NONE)` (atlas 64×32), shared by the
// ocelot (`ModelLayers.OCELOT`, unscaled) and the cat (`ModelLayers.CAT`, the same mesh scaled 0.8 by
// `AdultCatModel.CAT_TRANSFORMER` — applied in the root transform). Eight root parts: the `head`
// (skull, nose, two ears), the pitched `body`, the two tail segments, and the four legs. The base
// `AdultFelineModel.setupAnim` sets `head.xRot/yRot` from the look, applies crouch/sprint body-tail
// offsets, swings the four legs with its own mirrored phase + amplitude-1.0 formula, and, while not
// sitting, drops the lower tail to `tail2.xRot = 1.7278761` plus the branch-specific
// `cos(pos)·speed` wobble (`π/4`, crouch `0.47123894`, sprint `π/10`). `isSitting` folds the
// body/tail/legs after head look and skips that not-sitting walk branch. `lieDownAmount`,
// `lieDownAmountTail`, and `relaxStateOneAmount` stay deferred. The textured path binds cat/ocelot
// textures and the tame-cat collar layer; the colored debug path remains a single tan tint.
// Cat/ocelot use a plain `MobRenderer`.

/// Vanilla `AdultCatModel.CAT_TRANSFORMER = MeshTransformer.scaling(0.8)`: the cat layer is the shared
/// feline mesh scaled 0.8. The ocelot layer is unscaled.
pub(in crate::entity_models) const FELINE_CAT_SCALE: f32 = 0.8;

// `head` cubes: the 5×4×5 skull (texOffs (0,0)), the 3×2×2 nose (texOffs (0,24)), and the two 1×1×2
// ears (texOffs (0,10) / (6,10) — distinct, not mirrors).
pub(in crate::entity_models) const FELINE_HEAD_CUBES: [ModelCube; 4] = [
    ModelCube::new(
        [-2.5, -2.0, -3.0],
        [5.0, 4.0, 5.0],
        FELINE_TAN,
        [5.0, 4.0, 5.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-1.5, -0.001, -4.0],
        [3.0, 2.0, 2.0],
        FELINE_TAN,
        [3.0, 2.0, 2.0],
        [0.0, 24.0],
        false,
    ),
    ModelCube::new(
        [-2.0, -3.0, 0.0],
        [1.0, 1.0, 2.0],
        FELINE_TAN,
        [1.0, 1.0, 2.0],
        [0.0, 10.0],
        false,
    ),
    ModelCube::new(
        [1.0, -3.0, 0.0],
        [1.0, 1.0, 2.0],
        FELINE_TAN,
        [1.0, 1.0, 2.0],
        [6.0, 10.0],
        false,
    ),
];

// `body`: the 4×16×6 trunk (pitched onto its belly), texOffs (20,0).
pub(in crate::entity_models) const FELINE_BODY_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 3.0, -8.0],
    [4.0, 16.0, 6.0],
    FELINE_TAN,
    [4.0, 16.0, 6.0],
    [20.0, 0.0],
    false,
)];

// `tail1`: the upper 1×8×1 tail segment, texOffs (0,15).
pub(in crate::entity_models) const FELINE_TAIL1_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-0.5, 0.0, 0.0],
    [1.0, 8.0, 1.0],
    FELINE_TAN,
    [1.0, 8.0, 1.0],
    [0.0, 15.0],
    false,
)];

// `tail2`: the lower 1×8×1 tail segment, deflated by the vanilla `CubeDeformation(-0.02)` (min += 0.02,
// size -= 0.04). texOffs (4,15); the UV box size stays the un-inflated [1,8,1] (the integer dx,dy,dz
// passed to `addBox`).
pub(in crate::entity_models) const FELINE_TAIL2_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-0.48, 0.02, 0.02],
    [0.96, 7.96, 0.96],
    FELINE_TAN,
    [1.0, 8.0, 1.0],
    [4.0, 15.0],
    false,
)];

// The two 2×6×2 hind legs (shared box, texOffs (8,13)), and the two 2×10×2 front legs (shared box,
// texOffs (40,0)). Left & right re-use the same box/texOffs (no mirror).
pub(in crate::entity_models) const FELINE_HIND_LEG_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, 1.0],
    [2.0, 6.0, 2.0],
    FELINE_TAN,
    [2.0, 6.0, 2.0],
    [8.0, 13.0],
    false,
)];
pub(in crate::entity_models) const FELINE_FRONT_LEG_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, 0.0],
    [2.0, 10.0, 2.0],
    FELINE_TAN,
    [2.0, 10.0, 2.0],
    [40.0, 0.0],
    false,
)];

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
                ModelPart::leaf(FELINE_HEAD_POSE, FELINE_HEAD_CUBES.to_vec()),
            ),
            (
                "body",
                ModelPart::leaf(FELINE_BODY_POSE, FELINE_BODY_CUBES.to_vec()),
            ),
            (
                "tail1",
                ModelPart::leaf(FELINE_TAIL1_POSE, FELINE_TAIL1_CUBES.to_vec()),
            ),
            (
                "tail2",
                ModelPart::leaf(FELINE_TAIL2_POSE, FELINE_TAIL2_CUBES.to_vec()),
            ),
            (
                "left_hind_leg",
                ModelPart::leaf(FELINE_LEFT_HIND_LEG_POSE, FELINE_HIND_LEG_CUBES.to_vec()),
            ),
            (
                "right_hind_leg",
                ModelPart::leaf(FELINE_RIGHT_HIND_LEG_POSE, FELINE_HIND_LEG_CUBES.to_vec()),
            ),
            (
                "left_front_leg",
                ModelPart::leaf(FELINE_LEFT_FRONT_LEG_POSE, FELINE_FRONT_LEG_CUBES.to_vec()),
            ),
            (
                "right_front_leg",
                ModelPart::leaf(FELINE_RIGHT_FRONT_LEG_POSE, FELINE_FRONT_LEG_CUBES.to_vec()),
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

// Baby `head` cubes (offset 0,20,-3.125): the 5×4×4 skull (texOffs (0,0)), two 1×1×2 ears (texOffs
// (18,0) / (24,0) — distinct, not mirrors), and a 3×2×1 nose (texOffs (18,3)).
pub(in crate::entity_models) const BABY_FELINE_HEAD_CUBES: [ModelCube; 4] = [
    ModelCube::new(
        [-2.5, -3.0, -2.875],
        [5.0, 4.0, 4.0],
        FELINE_TAN,
        [5.0, 4.0, 4.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-2.0, -4.0, -0.875],
        [1.0, 1.0, 2.0],
        FELINE_TAN,
        [1.0, 1.0, 2.0],
        [18.0, 0.0],
        false,
    ),
    ModelCube::new(
        [1.0, -4.0, -0.875],
        [1.0, 1.0, 2.0],
        FELINE_TAN,
        [1.0, 1.0, 2.0],
        [24.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-1.5, -1.0, -3.875],
        [3.0, 2.0, 1.0],
        FELINE_TAN,
        [3.0, 2.0, 1.0],
        [18.0, 3.0],
        false,
    ),
];

// Baby `body`: the 4×3×7 trunk (upright, not pitched), texOffs (0,8).
pub(in crate::entity_models) const BABY_FELINE_BODY_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-2.0, -1.5, -3.5],
    [4.0, 3.0, 7.0],
    FELINE_TAN,
    [4.0, 3.0, 7.0],
    [0.0, 8.0],
    false,
)];

// Baby legs: all four share the same 1×2×2 box, but — unlike the adult's shared-builder legs — each
// gets its OWN texOffs (no mirrors), so each leg carries a distinct cube const.
// `left_front_leg`: texOffs (18,18).
pub(in crate::entity_models) const BABY_FELINE_LEFT_FRONT_LEG_CUBES: [ModelCube; 1] =
    [ModelCube::new(
        [-0.5, 0.0, -1.0],
        [1.0, 2.0, 2.0],
        FELINE_TAN,
        [1.0, 2.0, 2.0],
        [18.0, 18.0],
        false,
    )];
// `right_front_leg`: texOffs (12,18).
pub(in crate::entity_models) const BABY_FELINE_RIGHT_FRONT_LEG_CUBES: [ModelCube; 1] =
    [ModelCube::new(
        [-0.5, 0.0, -1.0],
        [1.0, 2.0, 2.0],
        FELINE_TAN,
        [1.0, 2.0, 2.0],
        [12.0, 18.0],
        false,
    )];
// `left_hind_leg`: texOffs (18,22).
pub(in crate::entity_models) const BABY_FELINE_LEFT_HIND_LEG_CUBES: [ModelCube; 1] =
    [ModelCube::new(
        [-0.5, 0.0, -1.0],
        [1.0, 2.0, 2.0],
        FELINE_TAN,
        [1.0, 2.0, 2.0],
        [18.0, 22.0],
        false,
    )];
// `right_hind_leg`: texOffs (12,22).
pub(in crate::entity_models) const BABY_FELINE_RIGHT_HIND_LEG_CUBES: [ModelCube; 1] =
    [ModelCube::new(
        [-0.5, 0.0, -1.0],
        [1.0, 2.0, 2.0],
        FELINE_TAN,
        [1.0, 2.0, 2.0],
        [12.0, 22.0],
        false,
    )];

// Baby `tail1`: the single 1×1×5 tail segment (`tail2` below it is cubeless), texOffs (0,18).
pub(in crate::entity_models) const BABY_FELINE_TAIL1_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-0.5, -0.107, 0.0849],
    [1.0, 1.0, 5.0],
    FELINE_TAN,
    [1.0, 1.0, 5.0],
    [0.0, 18.0],
    false,
)];

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
                ModelPart::leaf(BABY_FELINE_HEAD_POSE, BABY_FELINE_HEAD_CUBES.to_vec()),
            ),
            (
                "left_front_leg",
                ModelPart::leaf(
                    BABY_FELINE_LEFT_FRONT_LEG_POSE,
                    BABY_FELINE_LEFT_FRONT_LEG_CUBES.to_vec(),
                ),
            ),
            (
                "right_front_leg",
                ModelPart::leaf(
                    BABY_FELINE_RIGHT_FRONT_LEG_POSE,
                    BABY_FELINE_RIGHT_FRONT_LEG_CUBES.to_vec(),
                ),
            ),
            (
                "left_hind_leg",
                ModelPart::leaf(
                    BABY_FELINE_LEFT_HIND_LEG_POSE,
                    BABY_FELINE_LEFT_HIND_LEG_CUBES.to_vec(),
                ),
            ),
            (
                "body",
                ModelPart::leaf(BABY_FELINE_BODY_POSE, BABY_FELINE_BODY_CUBES.to_vec()),
            ),
            (
                "right_hind_leg",
                ModelPart::leaf(
                    BABY_FELINE_RIGHT_HIND_LEG_POSE,
                    BABY_FELINE_RIGHT_HIND_LEG_CUBES.to_vec(),
                ),
            ),
            (
                "tail1",
                ModelPart::leaf(BABY_FELINE_TAIL1_POSE, BABY_FELINE_TAIL1_CUBES.to_vec()),
            ),
            ("tail2", ModelPart::leaf(BABY_FELINE_TAIL2_POSE, Vec::new())),
        ],
    )
}

/// Vanilla `AdultFelineModel.setupAnim` standing tail droop: while not sitting it sets
/// `tail2.xRot = 1.7278761` (`= π·0.55`), the base the walk wobble adds onto. At rest (zero walk
/// speed) the wobble term collapses to zero, so this is the resting `tail2` pitch — a real change
/// from the `0` bind rotation.
pub(in crate::entity_models) const FELINE_TAIL2_REST_X_ROT: f32 = 1.7278761;
const FELINE_TAIL2_CROUCH_WOBBLE_AMPLITUDE: f32 = 0.47123894;
pub(in crate::entity_models) const FELINE_SITTING_TAIL2_X_ROT: f32 = 2.670354;
pub(in crate::entity_models) const BABY_FELINE_SITTING_BODY_X_ROT_DELTA: f32 = -0.43633232;
pub(in crate::entity_models) const BABY_FELINE_SITTING_TAIL1_X_ROT_DELTA: f32 = 0.5454154;

/// Vanilla `AdultFelineModel.setupAnim` lower-tail walk wobble while not sitting:
/// `tail2.xRot = 1.7278761 + amplitude·cos(walkAnimationPos)·walkAnimationSpeed`.
/// The base droop [`FELINE_TAIL2_REST_X_ROT`] is the same constant the [`setup_anim`] standing droop
/// applies; the wobble is a pure function of the projected `walk_animation_pos` /
/// `walk_animation_speed`. The standing, crouch, and sprint branches keep the same base droop and only
/// swap the wobble amplitude (`π/4`, `0.47123894`, and `π/10`).
fn feline_tail2_wobble_x_rot(
    walk_animation_pos: f32,
    walk_animation_speed: f32,
    is_crouching: bool,
    is_sprinting: bool,
) -> f32 {
    let amplitude = if is_sprinting {
        std::f32::consts::PI / 10.0
    } else if is_crouching {
        FELINE_TAIL2_CROUCH_WOBBLE_AMPLITUDE
    } else {
        std::f32::consts::FRAC_PI_4
    };
    FELINE_TAIL2_REST_X_ROT + amplitude * walk_animation_pos.cos() * walk_animation_speed
}

/// Mutable feline model, mirroring vanilla `AdultFelineModel` / `BabyFelineModel` (ocelot and cat
/// share each). The named root parts hang off a synthetic root, built from the baked geometry;
/// the adult cat's 0.8 scale lives in the root transform ([`FELINE_CAT_SCALE`], applied by the
/// runtime — the babies are unscaled). `setup_anim` runs the head look
/// ([`apply_head_look`] on `child_mut("head")`), applies the crouch/sprint tail-body setup, swings the
/// legs with the gait, and, for the adult, drops and wobbles the lower tail via `child_mut("tail2")`;
/// sitting folds the vanilla branch for cats; lie-down / relax feline poses stay deferred.
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
/// at rest. The `tail2` walk wobble vanilla adds on top is applied by [`feline_tail2_wobble_x_rot`].
fn apply_feline_leg_swing(
    root: &mut ModelPart,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
    is_sprinting: bool,
) {
    if limb_swing_at_rest(walk_animation_speed) {
        return;
    }
    let phase = walk_animation_pos * 0.6662;
    let phases = if is_sprinting {
        [
            ("left_hind_leg", 0.0),
            ("right_hind_leg", 0.3),
            ("left_front_leg", std::f32::consts::PI + 0.3),
            ("right_front_leg", std::f32::consts::PI),
        ]
    } else {
        [
            ("left_hind_leg", 0.0),
            ("right_hind_leg", std::f32::consts::PI),
            ("left_front_leg", std::f32::consts::PI),
            ("right_front_leg", 0.0),
        ]
    };
    for (name, phase_offset) in phases {
        root.child_mut(name).pose.rotation[0] = (phase + phase_offset).cos() * walk_animation_speed;
    }
}

fn apply_feline_crouch_or_sprint_pose(
    root: &mut ModelPart,
    is_crouching: bool,
    is_sprinting: bool,
) {
    if is_crouching {
        root.child_mut("body").pose.offset[1] += 1.0;
        root.child_mut("head").pose.offset[1] += 2.0;
        root.child_mut("tail1").pose.offset[1] += 1.0;
        root.child_mut("tail2").pose.offset[1] -= 4.0;
        root.child_mut("tail2").pose.offset[2] += 2.0;
        root.child_mut("tail1").pose.rotation[0] = std::f32::consts::FRAC_PI_2;
        root.child_mut("tail2").pose.rotation[0] = std::f32::consts::FRAC_PI_2;
    } else if is_sprinting {
        let tail1_y = root.child_mut("tail1").pose.offset[1];
        root.child_mut("tail2").pose.offset[1] = tail1_y;
        root.child_mut("tail2").pose.offset[2] += 2.0;
        root.child_mut("tail1").pose.rotation[0] = std::f32::consts::FRAC_PI_2;
        root.child_mut("tail2").pose.rotation[0] = std::f32::consts::FRAC_PI_2;
    }
}

/// Vanilla `AdultFelineModel.setupAnim` sitting branch. The adult feline model is only used for adult
/// cats/ocelots, so `state.ageScale` is the vanilla adult baseline `1.0`.
fn apply_adult_feline_sitting_pose(root: &mut ModelPart) {
    root.child_mut("body").pose.rotation[0] = std::f32::consts::FRAC_PI_4;
    root.child_mut("body").pose.offset[1] -= 4.0;
    root.child_mut("body").pose.offset[2] += 5.0;
    root.child_mut("head").pose.offset[1] -= 3.3;
    root.child_mut("head").pose.offset[2] += 1.0;
    root.child_mut("tail1").pose.offset[1] += 8.0;
    root.child_mut("tail1").pose.offset[2] -= 2.0;
    root.child_mut("tail2").pose.offset[1] += 2.0;
    root.child_mut("tail2").pose.offset[2] -= 0.8;
    root.child_mut("tail1").pose.rotation[0] = FELINE_TAIL2_REST_X_ROT;
    root.child_mut("tail2").pose.rotation[0] = FELINE_SITTING_TAIL2_X_ROT;
    root.child_mut("left_front_leg").pose.rotation[0] = -std::f32::consts::PI / 20.0;
    root.child_mut("left_front_leg").pose.offset[1] += 2.0;
    root.child_mut("left_front_leg").pose.offset[2] -= 2.0;
    root.child_mut("right_front_leg").pose.rotation[0] = -std::f32::consts::PI / 20.0;
    root.child_mut("right_front_leg").pose.offset[1] += 2.0;
    root.child_mut("right_front_leg").pose.offset[2] -= 2.0;
    root.child_mut("left_hind_leg").pose.rotation[0] = -std::f32::consts::FRAC_PI_2;
    root.child_mut("left_hind_leg").pose.offset[1] += 3.0;
    root.child_mut("left_hind_leg").pose.offset[2] -= 4.0;
    root.child_mut("right_hind_leg").pose.rotation[0] = -std::f32::consts::FRAC_PI_2;
    root.child_mut("right_hind_leg").pose.offset[1] += 3.0;
    root.child_mut("right_hind_leg").pose.offset[2] -= 4.0;
}

/// Vanilla `BabyFelineModel.setupAnim` sitting branch. Baby feline applies these deltas unscaled after
/// the shared head-look and crouch/sprint setup.
fn apply_baby_feline_sitting_pose(root: &mut ModelPart) {
    root.child_mut("body").pose.rotation[0] += BABY_FELINE_SITTING_BODY_X_ROT_DELTA;
    root.child_mut("body").pose.offset[1] += 1.25;
    root.child_mut("head").pose.offset[2] += 0.75;
    root.child_mut("tail1").pose.rotation[0] += BABY_FELINE_SITTING_TAIL1_X_ROT_DELTA;
    root.child_mut("tail1").pose.offset[1] += 4.0;
    root.child_mut("tail1").pose.offset[2] -= 0.9;
    root.child_mut("left_hind_leg").pose.offset[2] -= 0.9;
    root.child_mut("right_hind_leg").pose.offset[2] -= 0.9;
}

fn apply_feline_sitting_pose(root: &mut ModelPart, baby: bool) {
    if baby {
        apply_baby_feline_sitting_pose(root);
    } else {
        apply_adult_feline_sitting_pose(root);
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
        apply_feline_crouch_or_sprint_pose(
            &mut self.root,
            render_state.feline_is_crouching,
            render_state.feline_is_sprinting,
        );
        apply_head_look(
            self.root.child_mut("head"),
            render_state.head_yaw,
            render_state.head_pitch,
        );
        if render_state.feline_is_sitting {
            apply_feline_sitting_pose(&mut self.root, self.baby);
            return;
        }
        // Vanilla's not-sitting branch drops the lower tail to `1.7278761` and then adds the
        // branch-specific standing/crouch/sprint wobble. At rest the wobble collapses to zero, leaving
        // the standing droop. The baby's `tail2` is cubeless, so vanilla's identical assignment there is
        // invisible; we skip it.
        if !self.baby {
            self.root.child_mut("tail2").pose.rotation[0] = feline_tail2_wobble_x_rot(
                render_state.walk_animation_pos,
                render_state.walk_animation_speed,
                render_state.feline_is_crouching,
                render_state.feline_is_sprinting,
            );
        }
        // The four legs sweep only in vanilla's not-sitting branch; sitting returned above.
        apply_feline_leg_swing(
            &mut self.root,
            render_state.walk_animation_pos,
            render_state.walk_animation_speed,
            render_state.feline_is_sprinting,
        );
    }
}
