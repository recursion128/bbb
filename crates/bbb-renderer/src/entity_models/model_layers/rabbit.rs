use super::super::keyframe::{
    degree_vec, keyframe, keyframe_animated_pose, keyframe_elapsed_seconds, pos_vec,
    sample_bone_offsets, AnimationChannel, AnimationDefinition, AnimationTarget, BoneAnimation,
    Keyframe, KeyframeInterpolation,
};
use super::{apply_head_look, PartPose, PART_POSE_ZERO, RABBIT_BROWN};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// Vanilla 26.1 `AdultRabbitModel.createBodyLayer` (atlas 64×64). The mesh root holds two parts: the
// `body` (carrying the tail, head — with the two ears — and the cubeless `frontlegs` pivot) and the
// cubeless `backlegs` pivot (carrying the two hind legs, each parenting a haunch). `RabbitModel.setupAnim`
// sets `head.xRot/yRot` from the plain look (overwriting the head's baked pitch, as vanilla assigns
// rather than adds) when the idle-head-tilt animation is not playing, then applies the looping
// `RabbitAnimation.HOP` additively over every bone while the rabbit is mid-jump ([`RABBIT_HOP`],
// event-`1`-driven via the projected `rabbit_hop_seconds`); the random-timed `IDLE_HEAD_TILT`
// keyframe (gated on a `random.nextInt(40) + 180` timeout) is not reconstructable and stays deferred,
// so a resting rabbit renders at this bind pose plus the head look. The seven `Rabbit.Variant` colors and
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
// shared bbb box becomes a per-side const here). NOTE the const labels are INVERTED vs vanilla:
// `RABBIT_RIGHT_EAR_*` (pivot X +1.5, texOffs(32, 0)) is geometrically vanilla's `left_ear`, and
// `RABBIT_LEFT_EAR_*` (pivot X -1.5, texOffs(26, 0)) is vanilla's `right_ear`. The child nodes are
// therefore named by their TRUE vanilla identity (so `RABBIT_HOP`'s asymmetric per-ear channels apply
// by name) — see [`adult_rabbit_root`]. UV is keyed by pivot.
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
    // `head`'s two ears are named by their TRUE vanilla identity so `RABBIT_HOP`'s asymmetric per-ear
    // channels apply by name: the `RABBIT_RIGHT_EAR_*` const (pivot +1.5, texOffs 32,0) is vanilla's
    // `left_ear`, and `RABBIT_LEFT_EAR_*` (pivot -1.5) is vanilla's `right_ear` (the const labels are
    // inverted — see the ear-cube note above). The const ORDER (+1.5 first) is preserved, so the
    // vertex emission is unchanged and matches vanilla's left-then-right child order.
    let head = ModelPart::new(
        RABBIT_HEAD_POSE,
        RABBIT_HEAD_CUBES.to_vec(),
        vec![
            (
                "left_ear",
                ModelPart::leaf(RABBIT_RIGHT_EAR_POSE, RABBIT_RIGHT_EAR_CUBES.to_vec()),
            ),
            (
                "right_ear",
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
    // The baby `head`'s two ears carry the vanilla `right_ear` / `left_ear` names so the shared
    // `RABBIT_HOP` keyframe's per-ear channels address them. Unlike the adult consts, the baby ear
    // consts are NOT inverted (`BABY_RABBIT_RIGHT_EAR_*` sits at the vanilla right pivot -1.5), so the
    // child name matches its const here. Order (right then left) preserves the vertex emission.
    let head = ModelPart::new(
        BABY_RABBIT_HEAD_POSE,
        BABY_RABBIT_HEAD_CUBES.to_vec(),
        vec![
            (
                "right_ear",
                ModelPart::leaf(
                    BABY_RABBIT_RIGHT_EAR_POSE,
                    BABY_RABBIT_RIGHT_EAR_CUBES.to_vec(),
                ),
            ),
            (
                "left_ear",
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

const CATMULLROM: KeyframeInterpolation = KeyframeInterpolation::CatmullRom;
const LINEAR: KeyframeInterpolation = KeyframeInterpolation::Linear;

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

// ----- `RabbitAnimation.HOP` (length 0.75s, LOOPING). The bunny-hop arc: the `body` rocks up then
// settles, the `head` counter-bobs, the `backlegs`/`frontlegs` groups and the four individual legs
// kick through the jump, and the two ears and tail flick (each bone ROTATION + POSITION). Vanilla
// mixes LINEAR and CATMULLROM keyframes; the all-zero POSITION channels on several bones are
// transcribed faithfully (they add nothing). `posVec` negates the y axis and `degreeVec` converts to
// radians. Driven by the projected `rabbit_hop_seconds`, wrapped by the 0.75s loop each frame. -----

const RABBIT_HOP_BODY_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2083, degree_vec(4.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.2917, degree_vec(32.5, 0.0, 0.0), LINEAR),
    keyframe(0.4167, degree_vec(33.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.5833, degree_vec(18.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.75, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const RABBIT_HOP_BODY_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.75, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const RABBIT_HOP_HEAD_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2083, degree_vec(-4.0, 0.0, 0.0), LINEAR),
    keyframe(0.2917, degree_vec(-32.17, 0.0, 0.0), LINEAR),
    keyframe(0.375, degree_vec(-34.58, 0.0, 0.0), LINEAR),
    keyframe(0.5833, degree_vec(-20.0, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const RABBIT_HOP_HEAD_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.375, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.75, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const RABBIT_HOP_BACKLEGS_ROT: [Keyframe; 9] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.25, degree_vec(125.0, 0.0, 0.0), LINEAR),
    keyframe(0.2917, degree_vec(125.0, 0.0, 0.0), LINEAR),
    keyframe(0.375, degree_vec(120.0, 0.0, 0.0), LINEAR),
    keyframe(0.4583, degree_vec(95.0, 0.0, 0.0), LINEAR),
    keyframe(0.5417, degree_vec(42.0, 0.0, 0.0), LINEAR),
    keyframe(0.6667, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const RABBIT_HOP_BACKLEGS_POS: [Keyframe; 3] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.6667, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.75, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const RABBIT_HOP_FRONTLEGS_ROT: [Keyframe; 8] = [
    keyframe(0.0, degree_vec(-0.17, 0.0, 0.0), LINEAR),
    keyframe(0.125, degree_vec(-0.17, 0.0, 0.0), LINEAR),
    keyframe(0.2083, degree_vec(25.25, 0.0, 0.0), LINEAR),
    keyframe(0.2917, degree_vec(-65.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.4583, degree_vec(-67.5, 0.0, 0.0), CATMULLROM),
    keyframe(0.625, degree_vec(-1.25, 0.0, 0.0), LINEAR),
    keyframe(0.749, degree_vec(-1.25, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const RABBIT_HOP_FRONTLEGS_POS: [Keyframe; 5] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.3333, pos_vec(0.0, 0.5, 0.6), LINEAR),
    keyframe(0.4167, pos_vec(0.0, 0.9, 0.4), LINEAR),
    keyframe(0.75, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const RABBIT_HOP_RIGHT_FRONT_LEG_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.125, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.2083, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.3333, degree_vec(0.0, 0.0, -17.5), CATMULLROM),
    keyframe(0.5, degree_vec(0.0, 0.0, -17.5), CATMULLROM),
    keyframe(0.5833, degree_vec(0.0, 0.0, -2.0), CATMULLROM),
    keyframe(0.75, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const RABBIT_HOP_RIGHT_FRONT_LEG_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2083, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.75, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const RABBIT_HOP_LEFT_FRONT_LEG_ROT: [Keyframe; 7] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.125, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.2083, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.3333, degree_vec(0.0, 0.0, 20.0), CATMULLROM),
    keyframe(0.5, degree_vec(0.0, 0.0, 20.0), CATMULLROM),
    keyframe(0.5833, degree_vec(0.0, 0.0, 2.0), CATMULLROM),
    keyframe(0.75, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const RABBIT_HOP_LEFT_FRONT_LEG_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2083, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.75, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const RABBIT_HOP_LEFT_EAR_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.125, degree_vec(2.5, 0.0, 0.0), CATMULLROM),
    keyframe(0.375, degree_vec(-48.5, 0.0, 0.0), CATMULLROM),
    keyframe(0.5417, degree_vec(-41.24, 0.0, 0.0), CATMULLROM),
    keyframe(0.75, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const RABBIT_HOP_LEFT_EAR_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(-0.025, 0.0, 0.0), CATMULLROM),
    keyframe(0.2083, pos_vec(-0.025, -0.2, 0.0), CATMULLROM),
    keyframe(0.375, pos_vec(-0.02, -0.3, 0.0), CATMULLROM),
    keyframe(0.75, pos_vec(-0.025, 0.0, 0.0), CATMULLROM),
];
const RABBIT_HOP_RIGHT_EAR_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.125, degree_vec(7.5, 0.0, 0.0), CATMULLROM),
    keyframe(0.375, degree_vec(-31.5, 0.0, 0.0), CATMULLROM),
    keyframe(0.5, degree_vec(-35.33, 0.0, 0.0), CATMULLROM),
    keyframe(0.75, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const RABBIT_HOP_RIGHT_EAR_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(0.025, 0.0, 0.0), CATMULLROM),
    keyframe(0.2083, pos_vec(0.025, -0.3, 0.0), CATMULLROM),
    keyframe(0.375, pos_vec(0.02, -0.23, 0.0), CATMULLROM),
    keyframe(0.75, pos_vec(0.025, 0.0, 0.0), CATMULLROM),
];
const RABBIT_HOP_RIGHT_HIND_LEG_ROT: [Keyframe; 6] = [
    keyframe(0.0, degree_vec(0.0, -2.5, 0.0), LINEAR),
    keyframe(0.1667, degree_vec(0.0, -2.5, 0.0), LINEAR),
    keyframe(0.2083, degree_vec(47.5, 0.0, 0.0), CATMULLROM),
    keyframe(0.4167, degree_vec(47.5, 0.0, 0.0), CATMULLROM),
    keyframe(0.4583, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(0.0, -2.5, 0.0), LINEAR),
];
const RABBIT_HOP_RIGHT_HIND_LEG_POS: [Keyframe; 3] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.6667, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.75, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const RABBIT_HOP_LEFT_HIND_LEG_ROT: [Keyframe; 6] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.1667, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.2083, degree_vec(47.5, 0.0, 0.0), LINEAR),
    keyframe(0.4167, degree_vec(47.5, 0.0, 0.0), CATMULLROM),
    keyframe(0.4583, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const RABBIT_HOP_LEFT_HIND_LEG_POS: [Keyframe; 3] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.6667, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.75, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const RABBIT_HOP_TAIL_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.125, degree_vec(-25.0, 0.0, 0.0), LINEAR),
    keyframe(0.3333, degree_vec(15.0, 0.0, 0.0), LINEAR),
    keyframe(0.375, degree_vec(27.5, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const RABBIT_HOP_TAIL_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.75, pos_vec(0.0, 0.0, 0.0), LINEAR),
];
const RABBIT_HOP_BODY_CHANNELS: [AnimationChannel; 2] =
    [rot(&RABBIT_HOP_BODY_ROT), pos(&RABBIT_HOP_BODY_POS)];
const RABBIT_HOP_HEAD_CHANNELS: [AnimationChannel; 2] =
    [rot(&RABBIT_HOP_HEAD_ROT), pos(&RABBIT_HOP_HEAD_POS)];
const RABBIT_HOP_BACKLEGS_CHANNELS: [AnimationChannel; 2] =
    [rot(&RABBIT_HOP_BACKLEGS_ROT), pos(&RABBIT_HOP_BACKLEGS_POS)];
const RABBIT_HOP_FRONTLEGS_CHANNELS: [AnimationChannel; 2] = [
    rot(&RABBIT_HOP_FRONTLEGS_ROT),
    pos(&RABBIT_HOP_FRONTLEGS_POS),
];
const RABBIT_HOP_RIGHT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&RABBIT_HOP_RIGHT_FRONT_LEG_ROT),
    pos(&RABBIT_HOP_RIGHT_FRONT_LEG_POS),
];
const RABBIT_HOP_LEFT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&RABBIT_HOP_LEFT_FRONT_LEG_ROT),
    pos(&RABBIT_HOP_LEFT_FRONT_LEG_POS),
];
const RABBIT_HOP_LEFT_EAR_CHANNELS: [AnimationChannel; 2] =
    [rot(&RABBIT_HOP_LEFT_EAR_ROT), pos(&RABBIT_HOP_LEFT_EAR_POS)];
const RABBIT_HOP_RIGHT_EAR_CHANNELS: [AnimationChannel; 2] = [
    rot(&RABBIT_HOP_RIGHT_EAR_ROT),
    pos(&RABBIT_HOP_RIGHT_EAR_POS),
];
const RABBIT_HOP_RIGHT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&RABBIT_HOP_RIGHT_HIND_LEG_ROT),
    pos(&RABBIT_HOP_RIGHT_HIND_LEG_POS),
];
const RABBIT_HOP_LEFT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&RABBIT_HOP_LEFT_HIND_LEG_ROT),
    pos(&RABBIT_HOP_LEFT_HIND_LEG_POS),
];
const RABBIT_HOP_TAIL_CHANNELS: [AnimationChannel; 2] =
    [rot(&RABBIT_HOP_TAIL_ROT), pos(&RABBIT_HOP_TAIL_POS)];
const RABBIT_HOP_BONES: [BoneAnimation; 11] = [
    BoneAnimation {
        bone: "body",
        channels: &RABBIT_HOP_BODY_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &RABBIT_HOP_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "backlegs",
        channels: &RABBIT_HOP_BACKLEGS_CHANNELS,
    },
    BoneAnimation {
        bone: "frontlegs",
        channels: &RABBIT_HOP_FRONTLEGS_CHANNELS,
    },
    BoneAnimation {
        bone: "right_front_leg",
        channels: &RABBIT_HOP_RIGHT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_front_leg",
        channels: &RABBIT_HOP_LEFT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_ear",
        channels: &RABBIT_HOP_LEFT_EAR_CHANNELS,
    },
    BoneAnimation {
        bone: "right_ear",
        channels: &RABBIT_HOP_RIGHT_EAR_CHANNELS,
    },
    BoneAnimation {
        bone: "right_hind_leg",
        channels: &RABBIT_HOP_RIGHT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_hind_leg",
        channels: &RABBIT_HOP_LEFT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "tail",
        channels: &RABBIT_HOP_TAIL_CHANNELS,
    },
];

/// Vanilla `RabbitAnimation.HOP`: the 0.75s LOOPING bunny-hop (one loop = the 15-tick `jumpDuration`
/// window), `hopAnimation.apply(hopAnimationState, ageInTicks)`. Seeded by entity event `1` (the
/// jump), it runs for exactly one window per jump. The renderer applies it additively over the
/// look/bind pose only while the projected `rabbit_hop_seconds >= 0`, wrapping it by the loop length.
pub(in crate::entity_models) const RABBIT_HOP: AnimationDefinition = AnimationDefinition {
    length_seconds: 0.75,
    looping: true,
    bones: &RABBIT_HOP_BONES,
};

/// Mutable rabbit model, mirroring vanilla `AdultRabbitModel` / `BabyRabbitModel`. The two named root
/// parts hang off a synthetic root, built from the baked geometry for the selected `baby` layout.
/// `setup_anim` turns the body-nested `head` (`child_mut("body").child_mut("head")`) to the look
/// angles, then layers the looping `RABBIT_HOP` keyframe over every bone while the hop runs (the
/// random-timed idle-head-tilt keyframe stays deferred).
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
        // — and bbb never projects that random-timed `AnimationState`, so the look applies every frame.
        // The head is `body`'s child in both layouts.
        let head = self.root.child_mut("body").child_mut("head");
        apply_head_look(
            head,
            instance.render_state.head_yaw,
            instance.render_state.head_pitch,
        );

        // Vanilla `RabbitModel.setupAnim` then applies the looping `RabbitAnimation.HOP` additively
        // (`hopAnimation.apply` folds `offsetPos`/`offsetRotation` onto every animated bone) over the
        // look/bind pose while the hop is running. The projected `rabbit_hop_seconds` (`-1.0` =
        // stopped) is wrapped by the 0.75s loop length; a resting rabbit applies nothing. The eleven
        // HOP bones live at fixed paths shared by both the adult and baby trees.
        let hop_seconds = instance.render_state.rabbit_hop_seconds;
        if hop_seconds >= 0.0 {
            let seconds = keyframe_elapsed_seconds(&RABBIT_HOP, hop_seconds);
            let apply = |part: &mut ModelPart, bone: &str| {
                let (position, rotation) = sample_bone_offsets(&RABBIT_HOP, bone, seconds, 1.0);
                part.pose = keyframe_animated_pose(part.pose, position, rotation);
            };
            let body = self.root.child_mut("body");
            apply(body, "body");
            {
                let head = body.child_mut("head");
                apply(head, "head");
                apply(head.child_mut("right_ear"), "right_ear");
                apply(head.child_mut("left_ear"), "left_ear");
            }
            apply(body.child_mut("tail"), "tail");
            {
                let frontlegs = body.child_mut("frontlegs");
                apply(frontlegs, "frontlegs");
                apply(frontlegs.child_mut("right_front_leg"), "right_front_leg");
                apply(frontlegs.child_mut("left_front_leg"), "left_front_leg");
            }
            let backlegs = self.root.child_mut("backlegs");
            apply(backlegs, "backlegs");
            apply(backlegs.child_mut("right_hind_leg"), "right_hind_leg");
            apply(backlegs.child_mut("left_hind_leg"), "left_hind_leg");
        }
    }
}
