use super::{PartPose, AXOLOTL_BODY, AXOLOTL_GILLS, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_AXOLOTL: &str = "minecraft:axolotl#main";
pub(in crate::entity_models) const MODEL_LAYER_AXOLOTL_BABY: &str = "minecraft:axolotl_baby#main";

// Vanilla 26.1 `AdultAxolotlModel` (atlas 64×64) / `BabyAxolotlModel` (atlas 32×32)
// `createBodyLayer`. The axolotl is one of the `AgeableMobRenderer` two-model entities: the synced
// `AgeableMob.DATA_BABY_ID` flag selects the baby body layer, which has its own smaller geometry
// and a different leg topology. The adult body parents the head (which parents the three gill
// planes), the four leg planes, and the tail fin; the baby wraps the body under a `root` bone at
// `offset(0, 24, 0)` and parents the legs/tail/head off the body. The adult `setupAnim` IS driven:
// the body yaw plus the five factor-blended procedural sub-animations (swimming, water-hovering,
// ground-crawling, lay-still, play-dead) and the mirror-leg copy, all from the four projected
// `BinaryAnimator` factors. The baby's keyframe swim/walk/idle animations stay deferred. The five
// color variants (`Axolotl.Variant`) live on the deferred texture-backed path, so the colored debug
// path renders the lucy (pink) body with one body tint and one gill tint.

// ----- Adult -----

// `body` (offset (0, 19.5, 5)): the 8×4×10 trunk plus a 0×5×9 dorsal fin plane.
pub(in crate::entity_models) const ADULT_AXOLOTL_BODY_CUBES: [ModelCube; 2] = [
    ModelCube::new(
        [-4.0, -2.0, -9.0],
        [8.0, 4.0, 10.0],
        AXOLOTL_BODY,
        [8.0, 4.0, 10.0],
        [0.0, 11.0],
        false,
    ),
    ModelCube::new(
        [0.0, -3.0, -8.0],
        [0.0, 5.0, 9.0],
        AXOLOTL_BODY,
        [0.0, 5.0, 9.0],
        [2.0, 17.0],
        false,
    ),
];

// `head` (offset (0, 0, -9)): the 8×5×5 skull (`CubeDeformation(0.001)` fudge baked in).
pub(in crate::entity_models) const ADULT_AXOLOTL_HEAD_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-4.001, -3.001, -5.001],
    [8.002, 5.002, 5.002],
    AXOLOTL_BODY,
    [8.0, 5.0, 5.0],
    [0.0, 1.0],
    false,
)];

// The three gill planes (top 8×3×0, and the two 3×7×0 side frills), all fudge-inflated.
pub(in crate::entity_models) const ADULT_AXOLOTL_TOP_GILLS_CUBES: [ModelCube; 1] =
    [ModelCube::new(
        [-4.001, -3.001, -0.001],
        [8.002, 3.002, 0.002],
        AXOLOTL_GILLS,
        [8.0, 3.0, 0.0],
        [3.0, 37.0],
        false,
    )];
pub(in crate::entity_models) const ADULT_AXOLOTL_LEFT_GILLS_CUBES: [ModelCube; 1] =
    [ModelCube::new(
        [-3.001, -5.001, -0.001],
        [3.002, 7.002, 0.002],
        AXOLOTL_GILLS,
        [3.0, 7.0, 0.0],
        [0.0, 40.0],
        false,
    )];
pub(in crate::entity_models) const ADULT_AXOLOTL_RIGHT_GILLS_CUBES: [ModelCube; 1] =
    [ModelCube::new(
        [-0.001, -5.001, -0.001],
        [3.002, 7.002, 0.002],
        AXOLOTL_GILLS,
        [3.0, 7.0, 0.0],
        [11.0, 40.0],
        false,
    )];

// The 3×5×0 leg planes — the right legs use the `-2` origin, the left legs the `-1` origin.
pub(in crate::entity_models) const ADULT_AXOLOTL_RIGHT_LEG_CUBES: [ModelCube; 1] =
    [ModelCube::new(
        [-2.001, -0.001, -0.001],
        [3.002, 5.002, 0.002],
        AXOLOTL_BODY,
        [3.0, 5.0, 0.0],
        [2.0, 13.0],
        false,
    )];
pub(in crate::entity_models) const ADULT_AXOLOTL_LEFT_LEG_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-1.001, -0.001, -0.001],
    [3.002, 5.002, 0.002],
    AXOLOTL_BODY,
    [3.0, 5.0, 0.0],
    [2.0, 13.0],
    false,
)];

// `tail` (offset (0, 0, 1)): the 0×5×12 tail fin plane.
pub(in crate::entity_models) const ADULT_AXOLOTL_TAIL_CUBES: [ModelCube; 1] = [ModelCube::new(
    [0.0, -3.0, 0.0],
    [0.0, 5.0, 12.0],
    AXOLOTL_BODY,
    [0.0, 5.0, 12.0],
    [2.0, 19.0],
    false,
)];

/// `body` part pose: `PartPose.offset(0, 19.5, 5)`. The body parents the head (with the three gills),
/// the four leg planes, and the tail fin; only the `body` itself is turned (by yaw) in `setup_anim`,
/// so its children ride along index-named.
pub(in crate::entity_models) const ADULT_AXOLOTL_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 19.5, 5.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `head` part pose: `PartPose.offset(0, 0, -9)`.
pub(in crate::entity_models) const ADULT_AXOLOTL_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, -9.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `top_gills` part pose: `PartPose.offset(0, -3, -1)`.
pub(in crate::entity_models) const ADULT_AXOLOTL_TOP_GILLS_POSE: PartPose = PartPose {
    offset: [0.0, -3.0, -1.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_gills` part pose: `PartPose.offset(-4, 0, -1)`.
pub(in crate::entity_models) const ADULT_AXOLOTL_LEFT_GILLS_POSE: PartPose = PartPose {
    offset: [-4.0, 0.0, -1.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_gills` part pose: `PartPose.offset(4, 0, -1)`.
pub(in crate::entity_models) const ADULT_AXOLOTL_RIGHT_GILLS_POSE: PartPose = PartPose {
    offset: [4.0, 0.0, -1.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_front_leg` part pose: `PartPose.offset(-3.5, 1, -1)`.
pub(in crate::entity_models) const ADULT_AXOLOTL_RIGHT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [-3.5, 1.0, -1.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_front_leg` part pose: `PartPose.offset(3.5, 1, -1)`.
pub(in crate::entity_models) const ADULT_AXOLOTL_LEFT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [3.5, 1.0, -1.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_hind_leg` part pose: `PartPose.offset(-3.5, 1, -8)`.
pub(in crate::entity_models) const ADULT_AXOLOTL_RIGHT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [-3.5, 1.0, -8.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_hind_leg` part pose: `PartPose.offset(3.5, 1, -8)`.
pub(in crate::entity_models) const ADULT_AXOLOTL_LEFT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [3.5, 1.0, -8.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `tail` part pose: `PartPose.offset(0, 0, 1)`.
pub(in crate::entity_models) const ADULT_AXOLOTL_TAIL_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, 1.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Builds the adult axolotl's cube-bearing `body` part (parenting the index-named head — itself
/// parenting the three gill planes — the four leg planes, and the tail fin), in vanilla order.
fn adult_axolotl_body() -> ModelPart {
    let head = ModelPart::new(
        ADULT_AXOLOTL_HEAD_POSE,
        ADULT_AXOLOTL_HEAD_CUBES.to_vec(),
        vec![
            (
                "0",
                ModelPart::leaf(
                    ADULT_AXOLOTL_TOP_GILLS_POSE,
                    ADULT_AXOLOTL_TOP_GILLS_CUBES.to_vec(),
                ),
            ),
            (
                "1",
                ModelPart::leaf(
                    ADULT_AXOLOTL_LEFT_GILLS_POSE,
                    ADULT_AXOLOTL_LEFT_GILLS_CUBES.to_vec(),
                ),
            ),
            (
                "2",
                ModelPart::leaf(
                    ADULT_AXOLOTL_RIGHT_GILLS_POSE,
                    ADULT_AXOLOTL_RIGHT_GILLS_CUBES.to_vec(),
                ),
            ),
        ],
    );
    ModelPart::new(
        ADULT_AXOLOTL_BODY_POSE,
        ADULT_AXOLOTL_BODY_CUBES.to_vec(),
        vec![
            ("0", head),
            (
                "1",
                ModelPart::leaf(
                    ADULT_AXOLOTL_RIGHT_FRONT_LEG_POSE,
                    ADULT_AXOLOTL_RIGHT_LEG_CUBES.to_vec(),
                ),
            ),
            (
                "2",
                ModelPart::leaf(
                    ADULT_AXOLOTL_LEFT_FRONT_LEG_POSE,
                    ADULT_AXOLOTL_LEFT_LEG_CUBES.to_vec(),
                ),
            ),
            (
                "3",
                ModelPart::leaf(
                    ADULT_AXOLOTL_RIGHT_HIND_LEG_POSE,
                    ADULT_AXOLOTL_RIGHT_LEG_CUBES.to_vec(),
                ),
            ),
            (
                "4",
                ModelPart::leaf(
                    ADULT_AXOLOTL_LEFT_HIND_LEG_POSE,
                    ADULT_AXOLOTL_LEFT_LEG_CUBES.to_vec(),
                ),
            ),
            (
                "5",
                ModelPart::leaf(ADULT_AXOLOTL_TAIL_POSE, ADULT_AXOLOTL_TAIL_CUBES.to_vec()),
            ),
        ],
    )
}

// ----- Baby -----

// `body` (offset (0, -1.25, 1.75) under the root bone): the 4×2×6 trunk plus a 0×3×5 dorsal fin.
pub(in crate::entity_models) const BABY_AXOLOTL_BODY_CUBES: [ModelCube; 2] = [
    ModelCube::new(
        [-2.0, -0.75, -2.75],
        [4.0, 2.0, 6.0],
        AXOLOTL_BODY,
        [4.0, 2.0, 6.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [0.0, -1.75, -2.75],
        [0.0, 3.0, 5.0],
        AXOLOTL_BODY,
        [0.0, 3.0, 5.0],
        [0.0, 12.0],
        false,
    ),
];

// The 3×0×1 horizontal leg planes (the right hind leg is a doubly-rotated pivot/cube pair).
pub(in crate::entity_models) const BABY_AXOLOTL_RIGHT_FRONT_LEG_CUBES: [ModelCube; 1] =
    [ModelCube::new(
        [-3.0, 0.0, -0.5],
        [3.0, 0.0, 1.0],
        AXOLOTL_BODY,
        [3.0, 0.0, 1.0],
        [20.0, 16.0],
        false,
    )];
pub(in crate::entity_models) const BABY_AXOLOTL_RIGHT_HIND_LEG_CUBES: [ModelCube; 1] =
    [ModelCube::new(
        [0.0, 0.0, -0.5],
        [3.0, 0.0, 1.0],
        AXOLOTL_BODY,
        [3.0, 0.0, 1.0],
        [20.0, 14.0],
        false,
    )];
pub(in crate::entity_models) const BABY_AXOLOTL_LEFT_FRONT_LEG_CUBES: [ModelCube; 1] =
    [ModelCube::new(
        [0.0, 0.0, -0.5],
        [3.0, 0.0, 1.0],
        AXOLOTL_BODY,
        [3.0, 0.0, 1.0],
        [20.0, 13.0],
        false,
    )];
pub(in crate::entity_models) const BABY_AXOLOTL_LEFT_HIND_LEG_CUBES: [ModelCube; 1] =
    [ModelCube::new(
        [0.0, 0.0, -0.5],
        [3.0, 0.0, 1.0],
        AXOLOTL_BODY,
        [3.0, 0.0, 1.0],
        [20.0, 14.0],
        false,
    )];

// `tail` (offset (0, -0.25, 3.25)): the 0×3×8 tail fin plane.
pub(in crate::entity_models) const BABY_AXOLOTL_TAIL_CUBES: [ModelCube; 1] = [ModelCube::new(
    [0.0, -1.5, -1.0],
    [0.0, 3.0, 8.0],
    AXOLOTL_BODY,
    [0.0, 3.0, 8.0],
    [10.0, 9.0],
    false,
)];

// `head` (offset (0, 0.25, -2.75)): the 6×3×4 skull.
pub(in crate::entity_models) const BABY_AXOLOTL_HEAD_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-3.0, -2.0, -4.0],
    [6.0, 3.0, 4.0],
    AXOLOTL_BODY,
    [6.0, 3.0, 4.0],
    [0.0, 8.0],
    false,
)];

// The three gill planes (two 3×5×0 side frills and the 6×3×0 top frill).
pub(in crate::entity_models) const BABY_AXOLOTL_LEFT_GILLS_CUBES: [ModelCube; 1] =
    [ModelCube::new(
        [0.0, -3.5, 0.0],
        [3.0, 5.0, 0.0],
        AXOLOTL_GILLS,
        [3.0, 5.0, 0.0],
        [20.0, 8.0],
        false,
    )];
pub(in crate::entity_models) const BABY_AXOLOTL_RIGHT_GILLS_CUBES: [ModelCube; 1] =
    [ModelCube::new(
        [-3.0, -3.5, 0.0],
        [3.0, 5.0, 0.0],
        AXOLOTL_GILLS,
        [3.0, 5.0, 0.0],
        [20.0, 3.0],
        false,
    )];
pub(in crate::entity_models) const BABY_AXOLOTL_TOP_GILLS_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-3.0, -3.0, 0.0],
    [6.0, 3.0, 0.0],
    AXOLOTL_GILLS,
    [6.0, 3.0, 0.0],
    [20.0, 0.0],
    false,
)];

/// Baby `root` bone pose: `PartPose.offset(0, 24, 0)`.
pub(in crate::entity_models) const BABY_AXOLOTL_ROOT_POSE: PartPose = PartPose {
    offset: [0.0, 24.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `body` part pose: `PartPose.offset(0, -1.25, 1.75)`.
pub(in crate::entity_models) const BABY_AXOLOTL_BODY_POSE: PartPose = PartPose {
    offset: [0.0, -1.25, 1.75],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `right_front_leg` part pose: `PartPose.offset(-2, 0.25, -1.25)`.
pub(in crate::entity_models) const BABY_AXOLOTL_RIGHT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [-2.0, 0.25, -1.25],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `right_hind_leg` bare pivot pose: `PartPose.offsetAndRotation(-2, 0.25, 1.75, 0, π/2, π/2)`.
pub(in crate::entity_models) const BABY_AXOLOTL_RIGHT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [-2.0, 0.25, 1.75],
    rotation: [0.0, 1.5708, 1.5708],
};
/// Baby `right_leg_r1` cube pose: `PartPose.offsetAndRotation(0, 0, 0, -π/2, 0, π/2)`.
pub(in crate::entity_models) const BABY_AXOLOTL_RIGHT_LEG_R1_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, 0.0],
    rotation: [-1.5708, 0.0, 1.5708],
};
/// Baby `left_front_leg` part pose: `PartPose.offset(2, 0.25, -1.25)`.
pub(in crate::entity_models) const BABY_AXOLOTL_LEFT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [2.0, 0.25, -1.25],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `left_hind_leg` part pose: `PartPose.offset(2, 0.25, 1.75)`.
pub(in crate::entity_models) const BABY_AXOLOTL_LEFT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [2.0, 0.25, 1.75],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `tail` part pose: `PartPose.offset(0, -0.25, 3.25)`.
pub(in crate::entity_models) const BABY_AXOLOTL_TAIL_POSE: PartPose = PartPose {
    offset: [0.0, -0.25, 3.25],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `head` part pose: `PartPose.offset(0, 0.25, -2.75)`.
pub(in crate::entity_models) const BABY_AXOLOTL_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 0.25, -2.75],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `left_gills` part pose: `PartPose.offset(3, -0.5, -2)`.
pub(in crate::entity_models) const BABY_AXOLOTL_LEFT_GILLS_POSE: PartPose = PartPose {
    offset: [3.0, -0.5, -2.0],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `right_gills` part pose: `PartPose.offset(-3, -0.5, -2)`.
pub(in crate::entity_models) const BABY_AXOLOTL_RIGHT_GILLS_POSE: PartPose = PartPose {
    offset: [-3.0, -0.5, -2.0],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `top_gills` part pose: `PartPose.offset(0, -2, -2)`.
pub(in crate::entity_models) const BABY_AXOLOTL_TOP_GILLS_POSE: PartPose = PartPose {
    offset: [0.0, -2.0, -2.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Builds the baby axolotl's `root` bone parenting the cube-bearing `body` (which parents the four
/// index-named leg pivots — the right hind leg being a doubly-rotated pivot/cube pair — the tail fin,
/// and the head with its three gill planes), in vanilla order.
fn baby_axolotl_root() -> ModelPart {
    let head = ModelPart::new(
        BABY_AXOLOTL_HEAD_POSE,
        BABY_AXOLOTL_HEAD_CUBES.to_vec(),
        vec![
            (
                "0",
                ModelPart::leaf(
                    BABY_AXOLOTL_LEFT_GILLS_POSE,
                    BABY_AXOLOTL_LEFT_GILLS_CUBES.to_vec(),
                ),
            ),
            (
                "1",
                ModelPart::leaf(
                    BABY_AXOLOTL_RIGHT_GILLS_POSE,
                    BABY_AXOLOTL_RIGHT_GILLS_CUBES.to_vec(),
                ),
            ),
            (
                "2",
                ModelPart::leaf(
                    BABY_AXOLOTL_TOP_GILLS_POSE,
                    BABY_AXOLOTL_TOP_GILLS_CUBES.to_vec(),
                ),
            ),
        ],
    );
    let right_hind_leg = ModelPart::new(
        BABY_AXOLOTL_RIGHT_HIND_LEG_POSE,
        Vec::new(),
        vec![(
            "right_leg_r1",
            ModelPart::leaf(
                BABY_AXOLOTL_RIGHT_LEG_R1_POSE,
                BABY_AXOLOTL_RIGHT_HIND_LEG_CUBES.to_vec(),
            ),
        )],
    );
    let body = ModelPart::new(
        BABY_AXOLOTL_BODY_POSE,
        BABY_AXOLOTL_BODY_CUBES.to_vec(),
        vec![
            (
                "0",
                ModelPart::leaf(
                    BABY_AXOLOTL_RIGHT_FRONT_LEG_POSE,
                    BABY_AXOLOTL_RIGHT_FRONT_LEG_CUBES.to_vec(),
                ),
            ),
            ("1", right_hind_leg),
            (
                "2",
                ModelPart::leaf(
                    BABY_AXOLOTL_LEFT_FRONT_LEG_POSE,
                    BABY_AXOLOTL_LEFT_FRONT_LEG_CUBES.to_vec(),
                ),
            ),
            (
                "3",
                ModelPart::leaf(
                    BABY_AXOLOTL_LEFT_HIND_LEG_POSE,
                    BABY_AXOLOTL_LEFT_HIND_LEG_CUBES.to_vec(),
                ),
            ),
            (
                "4",
                ModelPart::leaf(BABY_AXOLOTL_TAIL_POSE, BABY_AXOLOTL_TAIL_CUBES.to_vec()),
            ),
            ("5", head),
        ],
    );
    ModelPart::new(BABY_AXOLOTL_ROOT_POSE, Vec::new(), vec![("body", body)])
}

/// The per-part rotation/offset deltas the adult axolotl's factor-blended sub-animations accumulate
/// before they are added onto the reset bind pose, mirroring vanilla `AdultAxolotlModel.setupAnim`'s
/// `+=` onto the freshly-`resetPose`d parts. Each field is a delta (radians for rotations, model
/// units for `body_y`), summed across all five sub-animations in vanilla order so the mirror-leg copy
/// sees the final accumulated left-leg deltas.
#[derive(Default)]
struct AxolotlPose {
    /// `body` `[xRot, _, zRot]` deltas (the `yRot` head-look is applied separately).
    body: [f32; 3],
    /// `body` y-offset delta (vanilla `this.body.y -= …`).
    body_y: f32,
    head: [f32; 3],
    tail_y: f32,
    top_gills_x: f32,
    left_gills_y: f32,
    right_gills_y: f32,
    left_hind: [f32; 3],
    left_front: [f32; 3],
    right_hind: [f32; 3],
    right_front: [f32; 3],
}

/// Vanilla `AdultAxolotlModel.setupSwimmingAnimation`: the moving-in-water gallop.
fn axolotl_setup_swimming(pose: &mut AxolotlPose, age: f32, x_rot_deg: f32, factor: f32) {
    if factor <= 1.0e-5 {
        return;
    }
    let anim_move_speed = age * 0.33;
    let sine_sway = anim_move_speed.sin();
    let cosine_sway = anim_move_speed.cos();
    let body_sway = 0.13 * sine_sway;
    pose.body[0] += (x_rot_deg.to_radians() + body_sway) * factor;
    pose.head[0] -= body_sway * 1.8 * factor;
    pose.body_y -= 0.45 * cosine_sway * factor;
    pose.top_gills_x += (-0.5 * sine_sway - 0.8) * factor;
    let gill_y_rot = (0.3 * sine_sway + 0.9) * factor;
    pose.left_gills_y += gill_y_rot;
    pose.right_gills_y -= gill_y_rot;
    pose.tail_y += 0.3 * (anim_move_speed * 0.9).cos() * factor;
    pose.left_hind[0] += 1.884_955_8 * factor;
    pose.left_hind[1] += -0.4 * sine_sway * factor;
    pose.left_hind[2] += std::f32::consts::FRAC_PI_2 * factor;
    pose.left_front[0] += 1.884_955_8 * factor;
    pose.left_front[1] += (-0.2 * cosine_sway - 0.1) * factor;
    pose.left_front[2] += std::f32::consts::FRAC_PI_2 * factor;
}

/// Vanilla `AdultAxolotlModel.setupWaterHoveringAnimation`: the still-in-water hover.
fn axolotl_setup_water_hovering(pose: &mut AxolotlPose, age: f32, factor: f32) {
    if factor <= 1.0e-5 {
        return;
    }
    let anim_move_speed = age * 0.075;
    let cosine_sway = anim_move_speed.cos();
    let sine_sway = anim_move_speed.sin() * 0.15;
    let body_x_rot = (-0.15 + 0.075 * cosine_sway) * factor;
    pose.body[0] += body_x_rot;
    pose.body_y -= sine_sway * factor;
    pose.head[0] -= body_x_rot;
    pose.top_gills_x += 0.2 * cosine_sway * factor;
    let gill_y_rot = (-0.3 * cosine_sway - 0.19) * factor;
    pose.left_gills_y += gill_y_rot;
    pose.right_gills_y -= gill_y_rot;
    pose.left_hind[0] += (3.0 * std::f32::consts::PI / 4.0 - cosine_sway * 0.11) * factor;
    pose.left_hind[1] += 0.471_238_94 * factor;
    pose.left_hind[2] += 1.727_876_1 * factor;
    pose.left_front[0] += (std::f32::consts::FRAC_PI_4 - cosine_sway * 0.2) * factor;
    pose.left_front[1] += 2.042_035 * factor;
    pose.tail_y += 0.5 * cosine_sway * factor;
}

/// Vanilla `AdultAxolotlModel.setupGroundCrawlingAnimation`: the moving-on-ground crawl.
fn axolotl_setup_ground_crawling(pose: &mut AxolotlPose, age: f32, factor: f32) {
    if factor <= 1.0e-5 {
        return;
    }
    let anim_move_speed = age * 0.11;
    let cosine_sway = anim_move_speed.cos();
    let hind_leg_y_rot_sway = (cosine_sway * cosine_sway - 2.0 * cosine_sway) / 5.0;
    let front_leg_y_rot_sway = 0.7 * cosine_sway;
    let head_and_tail_y_rot = 0.09 * cosine_sway * factor;
    pose.head[1] += head_and_tail_y_rot;
    pose.tail_y += head_and_tail_y_rot;
    let gill_angle =
        (0.6 - 0.08 * (cosine_sway * cosine_sway + 2.0 * anim_move_speed.sin())) * factor;
    pose.top_gills_x += gill_angle;
    pose.left_gills_y -= gill_angle;
    pose.right_gills_y += gill_angle;
    let hind_leg_x_rot = 0.942_477_9 * factor;
    let front_leg_x_rot = 1.099_557_4 * factor;
    pose.left_hind[0] += hind_leg_x_rot;
    pose.left_hind[1] += (1.5 - hind_leg_y_rot_sway) * factor;
    pose.left_hind[2] += -0.1 * factor;
    pose.left_front[0] += front_leg_x_rot;
    pose.left_front[1] += (std::f32::consts::FRAC_PI_2 - front_leg_y_rot_sway) * factor;
    pose.right_hind[0] += hind_leg_x_rot;
    pose.right_hind[1] += (-1.0 - hind_leg_y_rot_sway) * factor;
    pose.right_front[0] += front_leg_x_rot;
    pose.right_front[1] += (-std::f32::consts::FRAC_PI_2 - front_leg_y_rot_sway) * factor;
}

/// Vanilla `AdultAxolotlModel.setupLayStillOnGroundAnimation`: the still-on-ground rest.
fn axolotl_setup_lay_still(pose: &mut AxolotlPose, age: f32, factor: f32) {
    if factor <= 1.0e-5 {
        return;
    }
    let anim_move_speed = age * 0.09;
    let sine_sway = anim_move_speed.sin();
    let cosine_sway = anim_move_speed.cos();
    let movement = sine_sway * sine_sway - 2.0 * sine_sway;
    let movement2 = cosine_sway * cosine_sway - 3.0 * sine_sway;
    pose.head[0] += -0.09 * movement * factor;
    pose.head[2] += -0.2 * factor;
    pose.tail_y += (-0.1 + 0.1 * movement) * factor;
    let gill_angle = (0.6 + 0.05 * movement2) * factor;
    pose.top_gills_x += gill_angle;
    pose.left_gills_y -= gill_angle;
    pose.right_gills_y += gill_angle;
    pose.left_hind[0] += 1.1 * factor;
    pose.left_hind[1] += 1.0 * factor;
    pose.left_front[0] += 0.8 * factor;
    pose.left_front[1] += 2.3 * factor;
    pose.left_front[2] -= 0.5 * factor;
}

/// Vanilla `AdultAxolotlModel.setupPlayDeadAnimation`: the limp-on-its-side play-dead pose.
fn axolotl_setup_play_dead(pose: &mut AxolotlPose, factor: f32) {
    if factor <= 1.0e-5 {
        return;
    }
    pose.left_hind[0] += 1.413_716_7 * factor;
    pose.left_hind[1] += 1.099_557_4 * factor;
    pose.left_hind[2] += std::f32::consts::FRAC_PI_4 * factor;
    pose.left_front[0] += std::f32::consts::FRAC_PI_4 * factor;
    pose.left_front[1] += 2.042_035 * factor;
    pose.body[0] += -0.15 * factor;
    pose.body[2] += 0.35 * factor;
}

/// Vanilla `AdultAxolotlModel.applyMirrorLegRotations`: copy the accumulated left-leg rotations onto
/// the right legs (Y/Z mirrored) scaled by the `mirroredLegsFactor`. Runs last, so it reads the final
/// left-leg deltas from every preceding sub-animation.
fn axolotl_apply_mirror_legs(pose: &mut AxolotlPose, factor: f32) {
    if factor <= 1.0e-5 {
        return;
    }
    pose.right_hind[0] += pose.left_hind[0] * factor;
    pose.right_hind[1] += -pose.left_hind[1] * factor;
    pose.right_hind[2] += -pose.left_hind[2] * factor;
    pose.right_front[0] += pose.left_front[0] * factor;
    pose.right_front[1] += -pose.left_front[1] * factor;
    pose.right_front[2] += -pose.left_front[2] * factor;
}

/// Adds an `[xRot, yRot, zRot]` delta onto a part's reset bind rotation.
fn add_rotation(part: &mut ModelPart, delta: [f32; 3]) {
    part.pose.rotation[0] += delta[0];
    part.pose.rotation[1] += delta[1];
    part.pose.rotation[2] += delta[2];
}

/// Mutable axolotl model, mirroring vanilla `AdultAxolotlModel` / `BabyAxolotlModel`. The single
/// `body` root (adult) or `root → body` bone (baby), with its nested index-named hierarchy, hangs off
/// a synthetic root, built from the baked adult/baby colored geometry selected at construction.
/// Colored-only: `setup_anim` turns the adult body toward the look yaw via `child_mut("body")` (the
/// blended procedural sways, the mirror-leg copy, and the baby keyframe animations stay deferred).
pub(in crate::entity_models) struct AxolotlModel {
    root: ModelPart,
    baby: bool,
}

impl AxolotlModel {
    pub(in crate::entity_models) fn new(baby: bool) -> Self {
        let body_root = if baby {
            baby_axolotl_root()
        } else {
            adult_axolotl_body()
        };
        let name = if baby { "root" } else { "body" };
        Self {
            root: ModelPart::new(PART_POSE_ZERO, Vec::new(), vec![(name, body_root)]),
            baby,
        }
    }
}

impl EntityModel for AxolotlModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        // The baby keyframe swim/walk/idle animations stay deferred, so only the adult body is
        // animated (vanilla `BabyAxolotlModel` is a separate keyframe model).
        if self.baby {
            return;
        }
        let state = &instance.render_state;
        let age = state.age_in_ticks;
        let playing_dead = state.axolotl_playing_dead_factor;
        let in_water = state.axolotl_in_water_factor;
        let on_ground = state.axolotl_on_ground_factor;
        let moving = state.axolotl_moving_factor;
        let not_moving = 1.0 - moving;
        // Vanilla `AdultAxolotlModel.setupAnim`: `mirroredLegsFactor = 1 - min(onGround, moving)`.
        let mirrored_legs = 1.0 - on_ground.min(moving);

        // Accumulate every sub-animation's `+=` deltas (vanilla order) before touching the tree, so
        // the borrow checker is happy and the mirror-leg copy sees the final left-leg totals.
        let mut pose = AxolotlPose::default();
        axolotl_setup_swimming(&mut pose, age, state.head_pitch, moving.min(in_water));
        axolotl_setup_water_hovering(&mut pose, age, not_moving.min(in_water));
        axolotl_setup_ground_crawling(&mut pose, age, moving.min(on_ground));
        axolotl_setup_lay_still(&mut pose, age, not_moving.min(on_ground));
        axolotl_setup_play_dead(&mut pose, playing_dead);
        axolotl_apply_mirror_legs(&mut pose, mirrored_legs);

        // Apply onto the reset bind pose. Vanilla turns the whole body toward the look yaw
        // (`body.yRot += yRot·π/180`) before the sways; that `+=` collapses to the bind pose at a
        // level gaze. The body child indices are `0`=head, `1`=right_front, `2`=left_front,
        // `3`=right_hind, `4`=left_hind, `5`=tail; the head children are `0`=top_gills,
        // `1`=left_gills, `2`=right_gills.
        let head_yaw = state.head_yaw.to_radians();
        let body = self.root.child_mut("body");
        body.pose.rotation[0] += pose.body[0];
        body.pose.rotation[1] += head_yaw;
        body.pose.rotation[2] += pose.body[2];
        body.pose.offset[1] += pose.body_y;
        {
            let head = body.child_mut("0");
            head.pose.rotation[0] += pose.head[0];
            head.pose.rotation[1] += pose.head[1];
            head.pose.rotation[2] += pose.head[2];
            head.child_mut("0").pose.rotation[0] += pose.top_gills_x;
            head.child_mut("1").pose.rotation[1] += pose.left_gills_y;
            head.child_mut("2").pose.rotation[1] += pose.right_gills_y;
        }
        add_rotation(body.child_mut("1"), pose.right_front);
        add_rotation(body.child_mut("2"), pose.left_front);
        add_rotation(body.child_mut("3"), pose.right_hind);
        add_rotation(body.child_mut("4"), pose.left_hind);
        body.child_mut("5").pose.rotation[1] += pose.tail_y;
    }
}
