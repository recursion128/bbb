use super::{apply_head_look, PartPose, PART_POSE_ZERO, RABBIT_BROWN};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// Vanilla 26.1 `AdultRabbitModel.createBodyLayer` (atlas 64×64). The mesh root holds two parts: the
// `body` (carrying the tail, head — with the two ears — and the cubeless `frontlegs` pivot) and the
// cubeless `backlegs` pivot (carrying the two hind legs, each parenting a haunch). `RabbitModel.setupAnim`
// sets `head.xRot/yRot` from the plain look (overwriting the head's baked pitch, as vanilla assigns
// rather than adds) when the idle-head-tilt animation is not playing; the looping `RabbitAnimation.HOP`
// and `IDLE_HEAD_TILT` keyframe animations need un-projected `AnimationState`s and stay deferred, so a
// resting rabbit renders at this bind pose plus the head look. The seven `Rabbit.Variant` colors and
// the baby body layer live on the textured / baby paths; the colored debug path renders one brown
// tint while each unified cube also carries the textured `uv_size` / `texOffs` / `mirror`. The two ears
// and the two haunches share their bbb box geometry but vanilla gives the left and right sides distinct
// `texOffs`, so the textured UV forces a per-side cube const (the geometry stays identical). Rabbit uses
// a plain `MobRenderer`/`LivingEntityRenderer.setupRotations`.

// `body`: the 8×6×10 torso, texOffs(0, 0).
pub(in crate::entity_models) const RABBIT_BODY_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-4.0, -6.0, -9.0],
    [8.0, 6.0, 10.0],
    RABBIT_BROWN,
    [8.0, 6.0, 10.0],
    [0.0, 0.0],
    false,
)];

// `tail`: the 4×4×4 puff, texOffs(20, 16).
pub(in crate::entity_models) const RABBIT_TAIL_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-2.0, -3.0084, -1.0125],
    [4.0, 4.0, 4.0],
    RABBIT_BROWN,
    [4.0, 4.0, 4.0],
    [20.0, 16.0],
    false,
)];

// `head`: the 5×5×5 skull, texOffs(0, 16).
pub(in crate::entity_models) const RABBIT_HEAD_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-2.5, -3.0, -4.0],
    [5.0, 5.0, 5.0],
    RABBIT_BROWN,
    [5.0, 5.0, 5.0],
    [0.0, 16.0],
    false,
)];

// The 2×5×1 ear box. The two ears share their geometry but carry distinct vanilla `texOffs` (so the
// shared bbb box becomes a per-side const here): bbb's `right_ear` child (pivot X +1.5, equal to
// vanilla's `left_ear` pivot) → texOffs(32, 0); bbb's `left_ear` child (pivot X -1.5, vanilla's
// `right_ear` pivot) → texOffs(26, 0). The naming is inverted vs vanilla; UV is keyed by pivot.
pub(in crate::entity_models) const RABBIT_RIGHT_EAR_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -4.2929, -0.1213],
    [2.0, 5.0, 1.0],
    RABBIT_BROWN,
    [2.0, 5.0, 1.0],
    [32.0, 0.0],
    false,
)];
pub(in crate::entity_models) const RABBIT_LEFT_EAR_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -4.2929, -0.1213],
    [2.0, 5.0, 1.0],
    RABBIT_BROWN,
    [2.0, 5.0, 1.0],
    [26.0, 0.0],
    false,
)];

// The two 2×4×2 front legs (distinct boxes — the right one's box is nudged on X, matching vanilla).
// Right front leg texOffs(36, 18); left front leg texOffs(44, 18).
pub(in crate::entity_models) const RABBIT_RIGHT_FRONT_LEG_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-0.9, -1.0, -0.9],
    [2.0, 4.0, 2.0],
    RABBIT_BROWN,
    [2.0, 4.0, 2.0],
    [36.0, 18.0],
    false,
)];
pub(in crate::entity_models) const RABBIT_LEFT_FRONT_LEG_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -1.0, -1.0],
    [2.0, 4.0, 2.0],
    RABBIT_BROWN,
    [2.0, 4.0, 2.0],
    [44.0, 18.0],
    false,
)];

// The 2×1×6 haunch box. The two haunches share their geometry but carry distinct vanilla `texOffs`
// (so the shared bbb box becomes a per-side const here): bbb's right haunch (yaw +0.3927) →
// texOffs(20, 24); bbb's left haunch (yaw -0.3927) → texOffs(36, 24). UV is keyed by the haunch yaw.
pub(in crate::entity_models) const RABBIT_RIGHT_HAUNCH_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -5.0],
    [2.0, 1.0, 6.0],
    RABBIT_BROWN,
    [2.0, 1.0, 6.0],
    [20.0, 24.0],
    false,
)];
pub(in crate::entity_models) const RABBIT_LEFT_HAUNCH_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -5.0],
    [2.0, 1.0, 6.0],
    RABBIT_BROWN,
    [2.0, 1.0, 6.0],
    [36.0, 24.0],
    false,
)];

/// Vanilla `AdultRabbitModel.createBodyLayer` rest-pose hierarchy: the `body` (pitched `-0.3927`)
/// and the cubeless `backlegs` pivot. Nine cubes (the `frontlegs`, `backlegs`, and the two hind-leg
/// parts are cubeless pivots).
/// `body` part pose: `PartPose.offsetAndRotation(0, 23, 4, -0.3927, 0, 0)`.
pub(in crate::entity_models) const RABBIT_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 23.0, 4.0],
    rotation: [-0.3927, 0.0, 0.0],
};
/// `tail` part pose: `PartPose.offset(0, -4.9916, 0.0125)`.
pub(in crate::entity_models) const RABBIT_TAIL_POSE: PartPose = PartPose {
    offset: [0.0, -4.9916, 0.0125],
    rotation: [0.0, 0.0, 0.0],
};
/// `head` part pose: `PartPose.offsetAndRotation(0, -5.2929, -8.1213, 0.3927, 0, 0)`.
pub(in crate::entity_models) const RABBIT_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, -5.2929, -8.1213],
    rotation: [0.3927, 0.0, 0.0],
};
/// `right_ear` part pose: `PartPose.offset(1.5, -3.7071, -0.8787)`.
pub(in crate::entity_models) const RABBIT_RIGHT_EAR_POSE: PartPose = PartPose {
    offset: [1.5, -3.7071, -0.8787],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_ear` part pose: `PartPose.offset(-1.5, -3.7071, -0.8787)`.
pub(in crate::entity_models) const RABBIT_LEFT_EAR_POSE: PartPose = PartPose {
    offset: [-1.5, -3.7071, -0.8787],
    rotation: [0.0, 0.0, 0.0],
};
/// `frontlegs` cubeless-pivot part pose: `PartPose.offset(0, -1.5349, -6.3108)`.
pub(in crate::entity_models) const RABBIT_FRONTLEGS_POSE: PartPose = PartPose {
    offset: [0.0, -1.5349, -6.3108],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_front_leg` part pose: `PartPose.offsetAndRotation(-2, 1.9239, 0.3827, 0.3927, 0, 0)`.
pub(in crate::entity_models) const RABBIT_RIGHT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [-2.0, 1.9239, 0.3827],
    rotation: [0.3927, 0.0, 0.0],
};
/// `left_front_leg` part pose: `PartPose.offsetAndRotation(2, 1.9239, 0.4827, 0.3927, 0, 0)`.
pub(in crate::entity_models) const RABBIT_LEFT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [2.0, 1.9239, 0.4827],
    rotation: [0.3927, 0.0, 0.0],
};
/// `backlegs` cubeless-pivot part pose: `PartPose.offset(0, 23, 4)`.
pub(in crate::entity_models) const RABBIT_BACKLEGS_POSE: PartPose = PartPose {
    offset: [0.0, 23.0, 4.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `right_hind_leg` cubeless-pivot part pose: `PartPose.offset(-3, 0.5, 0)`.
pub(in crate::entity_models) const RABBIT_RIGHT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [-3.0, 0.5, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `left_hind_leg` cubeless-pivot part pose: `PartPose.offset(3, 0.5, 0)`.
pub(in crate::entity_models) const RABBIT_LEFT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [3.0, 0.5, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// Right `haunch` part pose: `PartPose.offsetAndRotation(0, -0.5, 0, 0, 0.3927, 0)`.
pub(in crate::entity_models) const RABBIT_RIGHT_HAUNCH_POSE: PartPose = PartPose {
    offset: [0.0, -0.5, 0.0],
    rotation: [0.0, 0.3927, 0.0],
};
/// Left `haunch` part pose: `PartPose.offsetAndRotation(0, -0.5, 0, 0, -0.3927, 0)`.
pub(in crate::entity_models) const RABBIT_LEFT_HAUNCH_POSE: PartPose = PartPose {
    offset: [0.0, -0.5, 0.0],
    rotation: [0.0, -0.3927, 0.0],
};

/// Builds the adult rabbit's two named root parts under a synthetic root: the cube-bearing `body`
/// (parenting `tail`, the cube-bearing `head` with its two ears, and the cubeless `frontlegs` pivot)
/// and the cubeless `backlegs` pivot (parenting the two cubeless hind-leg pivots, each carrying its
/// haunch). The head is name-addressed by `setup_anim`, so `body` carries named children.
fn adult_rabbit_root() -> ModelPart {
    // `head`'s two ears were unnamed (index-named) children under the original `ModelPart::colored`,
    // so they keep the positional `"0"` / `"1"` names that `INDEX_CHILD_NAMES` produced.
    let head = ModelPart::new(
        RABBIT_HEAD_POSE,
        RABBIT_HEAD_CUBES.to_vec(),
        vec![
            (
                "0",
                ModelPart::leaf(RABBIT_RIGHT_EAR_POSE, RABBIT_RIGHT_EAR_CUBES.to_vec()),
            ),
            (
                "1",
                ModelPart::leaf(RABBIT_LEFT_EAR_POSE, RABBIT_LEFT_EAR_CUBES.to_vec()),
            ),
        ],
    );
    let frontlegs = ModelPart::new(
        RABBIT_FRONTLEGS_POSE,
        Vec::new(),
        vec![
            (
                "right_front_leg",
                ModelPart::leaf(
                    RABBIT_RIGHT_FRONT_LEG_POSE,
                    RABBIT_RIGHT_FRONT_LEG_CUBES.to_vec(),
                ),
            ),
            (
                "left_front_leg",
                ModelPart::leaf(
                    RABBIT_LEFT_FRONT_LEG_POSE,
                    RABBIT_LEFT_FRONT_LEG_CUBES.to_vec(),
                ),
            ),
        ],
    );
    let body = ModelPart::new(
        RABBIT_BODY_POSE,
        RABBIT_BODY_CUBES.to_vec(),
        vec![
            (
                "tail",
                ModelPart::leaf(RABBIT_TAIL_POSE, RABBIT_TAIL_CUBES.to_vec()),
            ),
            ("head", head),
            ("frontlegs", frontlegs),
        ],
    );
    let right_hind_leg = ModelPart::new(
        RABBIT_RIGHT_HIND_LEG_POSE,
        Vec::new(),
        vec![(
            "haunch",
            ModelPart::leaf(RABBIT_RIGHT_HAUNCH_POSE, RABBIT_RIGHT_HAUNCH_CUBES.to_vec()),
        )],
    );
    let left_hind_leg = ModelPart::new(
        RABBIT_LEFT_HIND_LEG_POSE,
        Vec::new(),
        vec![(
            "haunch",
            ModelPart::leaf(RABBIT_LEFT_HAUNCH_POSE, RABBIT_LEFT_HAUNCH_CUBES.to_vec()),
        )],
    );
    let backlegs = ModelPart::new(
        RABBIT_BACKLEGS_POSE,
        Vec::new(),
        vec![
            ("right_hind_leg", right_hind_leg),
            ("left_hind_leg", left_hind_leg),
        ],
    );
    ModelPart::new(
        PART_POSE_ZERO,
        Vec::new(),
        vec![("body", body), ("backlegs", backlegs)],
    )
}

// Vanilla `BabyRabbitModel.createBodyLayer` (atlas 32×32). A much more nested layout than the adult:
// every cube hangs off an `_r1` rotation-intermediate part, and the `body` lists `body_r1` / `tail` /
// `head` / `frontlegs` (so the head is `body`'s THIRD child). Nine cubes.

// `body_r1`: the 4×3×6 trunk, texOffs(0, 8).
pub(in crate::entity_models) const BABY_RABBIT_BODY_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-2.0, -2.0, -3.0],
    [4.0, 3.0, 6.0],
    RABBIT_BROWN,
    [4.0, 3.0, 6.0],
    [0.0, 8.0],
    false,
)];
// `tail_r1`: the 3×3×3 puff, texOffs(0, 21).
pub(in crate::entity_models) const BABY_RABBIT_TAIL_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-1.4, -2.0268, -1.0177],
    [3.0, 3.0, 3.0],
    RABBIT_BROWN,
    [3.0, 3.0, 3.0],
    [0.0, 21.0],
    false,
)];
// `head`: the 5×4×4 skull, texOffs(0, 0).
pub(in crate::entity_models) const BABY_RABBIT_HEAD_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-2.5, -3.0, -3.0],
    [5.0, 4.0, 4.0],
    RABBIT_BROWN,
    [5.0, 4.0, 4.0],
    [0.0, 0.0],
    false,
)];
// The 2×4×1 ear box. The two ears share their geometry but carry distinct vanilla `texOffs` (so the
// shared bbb box becomes a per-side const here): bbb's right ear (pivot offset [-1.5,…]) →
// texOffs(18, 0); bbb's left ear (pivot offset [1.5,…]) → texOffs(24, 0). UV is keyed by pivot.
pub(in crate::entity_models) const BABY_RABBIT_RIGHT_EAR_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -3.5, -0.5],
    [2.0, 4.0, 1.0],
    RABBIT_BROWN,
    [2.0, 4.0, 1.0],
    [18.0, 0.0],
    false,
)];
pub(in crate::entity_models) const BABY_RABBIT_LEFT_EAR_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -3.5, -0.5],
    [2.0, 4.0, 1.0],
    RABBIT_BROWN,
    [2.0, 4.0, 1.0],
    [24.0, 0.0],
    false,
)];
// The 1×3×1 front-leg `_r1` box. The two legs share their geometry but carry distinct vanilla
// `texOffs` (so the shared bbb box becomes a per-side const here): bbb's left front leg (built first)
// → texOffs(18, 8); bbb's right front leg (built second) → texOffs(14, 8). UV is keyed by build order.
pub(in crate::entity_models) const BABY_RABBIT_LEFT_FRONT_LEG_CUBES: [ModelCube; 1] =
    [ModelCube::new(
        [-0.5, -1.5, -0.5],
        [1.0, 3.0, 1.0],
        RABBIT_BROWN,
        [1.0, 3.0, 1.0],
        [18.0, 8.0],
        false,
    )];
pub(in crate::entity_models) const BABY_RABBIT_RIGHT_FRONT_LEG_CUBES: [ModelCube; 1] =
    [ModelCube::new(
        [-0.5, -1.5, -0.5],
        [1.0, 3.0, 1.0],
        RABBIT_BROWN,
        [1.0, 3.0, 1.0],
        [14.0, 8.0],
        false,
    )];
// The 2×1×3 haunch box. The two haunches share their geometry but carry distinct vanilla `texOffs`
// (so the shared bbb box becomes a per-side const here): bbb's left haunch (pivot offset [1,0,0.5],
// yaw -0.7854) → texOffs(10, 17); bbb's right haunch (pivot offset [0.5,0,-0.9], yaw +0.7854) →
// texOffs(0, 17). UV is keyed by pivot / yaw.
pub(in crate::entity_models) const BABY_RABBIT_LEFT_HAUNCH_CUBES: [ModelCube; 1] =
    [ModelCube::new(
        [-2.0, -0.5, 0.0],
        [2.0, 1.0, 3.0],
        RABBIT_BROWN,
        [2.0, 1.0, 3.0],
        [10.0, 17.0],
        false,
    )];
pub(in crate::entity_models) const BABY_RABBIT_RIGHT_HAUNCH_CUBES: [ModelCube; 1] =
    [ModelCube::new(
        [-2.0, -0.5, 0.0],
        [2.0, 1.0, 3.0],
        RABBIT_BROWN,
        [2.0, 1.0, 3.0],
        [0.0, 17.0],
        false,
    )];

/// Vanilla `BabyRabbitModel.createBodyLayer` rest-pose hierarchy: the cubeless `body` pivot (parenting
/// `body_r1` / `tail` / `head` / `frontlegs`) and the cubeless `backlegs` pivot. Nine cubes.
/// Baby `body` cubeless-pivot part pose: `PartPose.offset(0, 23, 1.6)`.
pub(in crate::entity_models) const BABY_RABBIT_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 23.0, 1.6],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `body_r1` part pose: `PartPose.offsetAndRotation(0, -2, -1.6, -0.5236, 0, 0)`.
pub(in crate::entity_models) const BABY_RABBIT_BODY_R1_POSE: PartPose = PartPose {
    offset: [0.0, -2.0, -1.6],
    rotation: [-0.5236, 0.0, 0.0],
};
/// Baby `tail` cubeless-pivot part pose: `PartPose.offset(0, -2.2, 2)`.
pub(in crate::entity_models) const BABY_RABBIT_TAIL_POSE: PartPose = PartPose {
    offset: [0.0, -2.2, 2.0],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `tail_r1` part pose: `PartPose.offsetAndRotation(-0.1, 0, 0, -0.5236, 0, 0)`.
pub(in crate::entity_models) const BABY_RABBIT_TAIL_R1_POSE: PartPose = PartPose {
    offset: [-0.1, 0.0, 0.0],
    rotation: [-0.5236, 0.0, 0.0],
};
/// Baby `head` part pose: `PartPose.offset(0, -5, -2.6)`.
pub(in crate::entity_models) const BABY_RABBIT_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, -5.0, -2.6],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `right_ear` part pose: `PartPose.offset(-1.5, -3.5, -0.5)`.
pub(in crate::entity_models) const BABY_RABBIT_RIGHT_EAR_POSE: PartPose = PartPose {
    offset: [-1.5, -3.5, -0.5],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `left_ear` part pose: `PartPose.offset(1.5, -3.5, -0.5)`.
pub(in crate::entity_models) const BABY_RABBIT_LEFT_EAR_POSE: PartPose = PartPose {
    offset: [1.5, -3.5, -0.5],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `frontlegs` cubeless-pivot part pose: `PartPose.offset(0, -2.5, -2.6)`.
pub(in crate::entity_models) const BABY_RABBIT_FRONTLEGS_POSE: PartPose = PartPose {
    offset: [0.0, -2.5, -2.6],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby left `front_leg` cubeless-pivot part pose: `PartPose.offsetAndRotation(1, 1, -0.5, 0.3927, 0, 0)`.
pub(in crate::entity_models) const BABY_RABBIT_LEFT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [1.0, 1.0, -0.5],
    rotation: [0.3927, 0.0, 0.0],
};
/// Baby right `front_leg` cubeless-pivot part pose: `PartPose.offsetAndRotation(-1, 1, -0.5, 0.3927, 0, 0)`.
pub(in crate::entity_models) const BABY_RABBIT_RIGHT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [-1.0, 1.0, -0.5],
    rotation: [0.3927, 0.0, 0.0],
};
/// Baby front-leg `_r1` part pose: `PartPose.offsetAndRotation(0, 1, 0, -0.3927, 0, 0)` (both reuse it).
pub(in crate::entity_models) const BABY_RABBIT_FRONT_LEG_R1_POSE: PartPose = PartPose {
    offset: [0.0, 1.0, 0.0],
    rotation: [-0.3927, 0.0, 0.0],
};
/// Baby `backlegs` cubeless-pivot part pose: `PartPose.offset(0, 23, 2)`.
pub(in crate::entity_models) const BABY_RABBIT_BACKLEGS_POSE: PartPose = PartPose {
    offset: [0.0, 23.0, 2.0],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `left_hind_leg` cubeless-pivot part pose: `PartPose.offsetAndRotation(1.5, 0.5, 0.5, 0, 3.1416, 0)`.
pub(in crate::entity_models) const BABY_RABBIT_LEFT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [1.5, 0.5, 0.5],
    rotation: [0.0, 3.1416, 0.0],
};
/// Baby `right_hind_leg` cubeless-pivot part pose: `PartPose.offsetAndRotation(-1.5, 0.5, 0.5, 0, 3.1416, 0)`.
pub(in crate::entity_models) const BABY_RABBIT_RIGHT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [-1.5, 0.5, 0.5],
    rotation: [0.0, 3.1416, 0.0],
};
/// Baby left `haunch` part pose: `PartPose.offsetAndRotation(1, 0, 0.5, 0, -0.7854, 0)`.
pub(in crate::entity_models) const BABY_RABBIT_LEFT_HAUNCH_POSE: PartPose = PartPose {
    offset: [1.0, 0.0, 0.5],
    rotation: [0.0, -0.7854, 0.0],
};
/// Baby right `haunch` part pose: `PartPose.offsetAndRotation(0.5, 0, -0.9, 0, 0.7854, 0)`.
pub(in crate::entity_models) const BABY_RABBIT_RIGHT_HAUNCH_POSE: PartPose = PartPose {
    offset: [0.5, 0.0, -0.9],
    rotation: [0.0, 0.7854, 0.0],
};

/// Builds the baby rabbit's two named root parts under a synthetic root: the cubeless `body` pivot
/// (parenting `body_r1`, the cubeless `tail` → `tail_r1`, the cube-bearing `head` with its ears, and the
/// cubeless `frontlegs` pivot) and the cubeless `backlegs` pivot. The head is name-addressed by
/// `setup_anim`, so `body` carries named children.
fn baby_rabbit_root() -> ModelPart {
    let tail = ModelPart::new(
        BABY_RABBIT_TAIL_POSE,
        Vec::new(),
        vec![(
            "tail_r1",
            ModelPart::leaf(BABY_RABBIT_TAIL_R1_POSE, BABY_RABBIT_TAIL_CUBES.to_vec()),
        )],
    );
    // The baby `head`'s two ears were likewise unnamed (index-named) children under the original
    // `ModelPart::colored`, so they keep the positional `"0"` / `"1"` names.
    let head = ModelPart::new(
        BABY_RABBIT_HEAD_POSE,
        BABY_RABBIT_HEAD_CUBES.to_vec(),
        vec![
            (
                "0",
                ModelPart::leaf(
                    BABY_RABBIT_RIGHT_EAR_POSE,
                    BABY_RABBIT_RIGHT_EAR_CUBES.to_vec(),
                ),
            ),
            (
                "1",
                ModelPart::leaf(
                    BABY_RABBIT_LEFT_EAR_POSE,
                    BABY_RABBIT_LEFT_EAR_CUBES.to_vec(),
                ),
            ),
        ],
    );
    let left_front_leg = ModelPart::new(
        BABY_RABBIT_LEFT_FRONT_LEG_POSE,
        Vec::new(),
        vec![(
            "left_front_leg_r1",
            ModelPart::leaf(
                BABY_RABBIT_FRONT_LEG_R1_POSE,
                BABY_RABBIT_LEFT_FRONT_LEG_CUBES.to_vec(),
            ),
        )],
    );
    let right_front_leg = ModelPart::new(
        BABY_RABBIT_RIGHT_FRONT_LEG_POSE,
        Vec::new(),
        vec![(
            "right_front_leg_r1",
            ModelPart::leaf(
                BABY_RABBIT_FRONT_LEG_R1_POSE,
                BABY_RABBIT_RIGHT_FRONT_LEG_CUBES.to_vec(),
            ),
        )],
    );
    let frontlegs = ModelPart::new(
        BABY_RABBIT_FRONTLEGS_POSE,
        Vec::new(),
        vec![
            ("left_front_leg", left_front_leg),
            ("right_front_leg", right_front_leg),
        ],
    );
    let body = ModelPart::new(
        BABY_RABBIT_BODY_POSE,
        Vec::new(),
        vec![
            (
                "body_r1",
                ModelPart::leaf(BABY_RABBIT_BODY_R1_POSE, BABY_RABBIT_BODY_CUBES.to_vec()),
            ),
            ("tail", tail),
            ("head", head),
            ("frontlegs", frontlegs),
        ],
    );
    let left_hind_leg = ModelPart::new(
        BABY_RABBIT_LEFT_HIND_LEG_POSE,
        Vec::new(),
        vec![(
            "haunch",
            ModelPart::leaf(
                BABY_RABBIT_LEFT_HAUNCH_POSE,
                BABY_RABBIT_LEFT_HAUNCH_CUBES.to_vec(),
            ),
        )],
    );
    let right_hind_leg = ModelPart::new(
        BABY_RABBIT_RIGHT_HIND_LEG_POSE,
        Vec::new(),
        vec![(
            "haunch",
            ModelPart::leaf(
                BABY_RABBIT_RIGHT_HAUNCH_POSE,
                BABY_RABBIT_RIGHT_HAUNCH_CUBES.to_vec(),
            ),
        )],
    );
    let backlegs = ModelPart::new(
        BABY_RABBIT_BACKLEGS_POSE,
        Vec::new(),
        vec![
            ("left_hind_leg", left_hind_leg),
            ("right_hind_leg", right_hind_leg),
        ],
    );
    ModelPart::new(
        PART_POSE_ZERO,
        Vec::new(),
        vec![("body", body), ("backlegs", backlegs)],
    )
}

/// Mutable rabbit model, mirroring vanilla `AdultRabbitModel` / `BabyRabbitModel`. The two named root
/// parts hang off a synthetic root, built from the baked geometry for the selected `baby` layout.
/// `setup_anim` turns the body-nested `head` (`child_mut("body").child_mut("head")`) to the look
/// angles (the hop / idle-head-tilt keyframes stay deferred).
pub(in crate::entity_models) struct RabbitModel {
    root: ModelPart,
}

impl RabbitModel {
    pub(in crate::entity_models) fn new(baby: bool) -> Self {
        Self {
            root: if baby {
                baby_rabbit_root()
            } else {
                adult_rabbit_root()
            },
        }
    }
}

impl EntityModel for RabbitModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        // Vanilla `RabbitModel.setupAnim` sets `head.yRot/xRot` from the plain look (an assignment
        // that overwrites the head's baked pitch) whenever the idle-head-tilt animation is not playing
        // — and bbb never projects that `AnimationState`, so the look applies every frame. The head is
        // `body`'s child in both layouts.
        let head = self.root.child_mut("body").child_mut("head");
        apply_head_look(
            head,
            instance.render_state.head_yaw,
            instance.render_state.head_pitch,
        );
    }
}
