use super::{PartPose, AXOLOTL_BODY, AXOLOTL_GILLS, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// Vanilla 26.1 `AdultAxolotlModel` (atlas 64×64) / `BabyAxolotlModel` (atlas 32×32)
// `createBodyLayer`. The axolotl is one of the `AgeableMobRenderer` two-model entities: the synced
// `AgeableMob.DATA_BABY_ID` flag selects the baby body layer, which has its own smaller geometry
// and a different leg topology. The adult body parents the head (which parents the three gill
// planes), the four leg planes, and the tail fin; the baby wraps the body under a `root` bone at
// `offset(0, 24, 0)` and parents the legs/tail/head off the body. Every `setupAnim` animation is
// deferred — the body yaw, the swimming / water-hovering / ground-crawling / lay-still procedural
// sways (adult) and the keyframe swim/walk/idle animations (baby), the play-dead pose, and the
// mirror-leg copy. The five color variants (`Axolotl.Variant`) live on the deferred texture-backed
// path, so the colored debug path renders the lucy (pink) body with one body tint and one gill
// tint. This is the non-animated rest pose.

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
        // Vanilla `AdultAxolotlModel.setupAnim` turns the whole body toward the look target
        // (`body.yRot += yRot·π/180`) unconditionally before the factor-blended sways; that body yaw
        // is `+=` onto the bind, so it collapses to the bind pose at a level gaze. The baby model
        // never applies it (its keyframe swims stay deferred), so only the adult body turns.
        if !self.baby {
            self.root.child_mut("body").pose.rotation[1] +=
                instance.render_state.head_yaw.to_radians();
        }
    }
}
