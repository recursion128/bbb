use super::{
    apply_head_look, model_cube as cube, ModelCubeDesc, PartPose, PART_POSE_ZERO, RABBIT_BROWN,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

// Vanilla 26.1 `AdultRabbitModel.createBodyLayer` (atlas 64×64). The mesh root holds two parts: the
// `body` (carrying the tail, head — with the two ears — and the cubeless `frontlegs` pivot) and the
// cubeless `backlegs` pivot (carrying the two hind legs, each parenting a haunch). `RabbitModel.setupAnim`
// sets `head.xRot/yRot` from the plain look (overwriting the head's baked pitch, as vanilla assigns
// rather than adds) when the idle-head-tilt animation is not playing; the looping `RabbitAnimation.HOP`
// and `IDLE_HEAD_TILT` keyframe animations need un-projected `AnimationState`s and stay deferred, so a
// resting rabbit renders at this bind pose plus the head look. The seven `Rabbit.Variant` colors and
// the baby body layer live on the deferred texture-backed / baby paths, so the colored debug path
// renders one brown tint. Colored-only (no textured path yet), so the cubes stay [`ModelCubeDesc`] and
// the tree is assembled imperatively from named children. Rabbit uses a plain
// `MobRenderer`/`LivingEntityRenderer.setupRotations`.

// `body`: the 8×6×10 torso.
pub(in crate::entity_models) const RABBIT_BODY_CUBES: [ModelCubeDesc; 1] =
    [cube([-4.0, -6.0, -9.0], [8.0, 6.0, 10.0], RABBIT_BROWN)];

// `tail`: the 4×4×4 puff.
pub(in crate::entity_models) const RABBIT_TAIL_CUBES: [ModelCubeDesc; 1] = [cube(
    [-2.0, -3.0084, -1.0125],
    [4.0, 4.0, 4.0],
    RABBIT_BROWN,
)];

// `head`: the 5×5×5 skull.
pub(in crate::entity_models) const RABBIT_HEAD_CUBES: [ModelCubeDesc; 1] =
    [cube([-2.5, -3.0, -4.0], [5.0, 5.0, 5.0], RABBIT_BROWN)];

// The shared 2×5×1 ear (both ears reuse it, differing only in pivot X sign).
pub(in crate::entity_models) const RABBIT_EAR_CUBES: [ModelCubeDesc; 1] = [cube(
    [-1.0, -4.2929, -0.1213],
    [2.0, 5.0, 1.0],
    RABBIT_BROWN,
)];

// The two 2×4×2 front legs (the right one's box is nudged on X, matching vanilla).
pub(in crate::entity_models) const RABBIT_RIGHT_FRONT_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-0.9, -1.0, -0.9], [2.0, 4.0, 2.0], RABBIT_BROWN)];
pub(in crate::entity_models) const RABBIT_LEFT_FRONT_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.0, -1.0, -1.0], [2.0, 4.0, 2.0], RABBIT_BROWN)];

// The shared 2×1×6 haunch (both reuse it, differing only in the haunch yaw sign).
pub(in crate::entity_models) const RABBIT_HAUNCH_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.0, 0.0, -5.0], [2.0, 1.0, 6.0], RABBIT_BROWN)];

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
    let head = ModelPart::colored(
        RABBIT_HEAD_POSE,
        &RABBIT_HEAD_CUBES,
        vec![
            ModelPart::leaf_colored(RABBIT_RIGHT_EAR_POSE, &RABBIT_EAR_CUBES),
            ModelPart::leaf_colored(RABBIT_LEFT_EAR_POSE, &RABBIT_EAR_CUBES),
        ],
    );
    let frontlegs = ModelPart::new(
        RABBIT_FRONTLEGS_POSE,
        Vec::new(),
        vec![
            (
                "right_front_leg",
                ModelPart::leaf_colored(RABBIT_RIGHT_FRONT_LEG_POSE, &RABBIT_RIGHT_FRONT_LEG_CUBES),
            ),
            (
                "left_front_leg",
                ModelPart::leaf_colored(RABBIT_LEFT_FRONT_LEG_POSE, &RABBIT_LEFT_FRONT_LEG_CUBES),
            ),
        ],
    );
    let body = ModelPart::colored_named(
        RABBIT_BODY_POSE,
        &RABBIT_BODY_CUBES,
        vec![
            (
                "tail",
                ModelPart::leaf_colored(RABBIT_TAIL_POSE, &RABBIT_TAIL_CUBES),
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
            ModelPart::leaf_colored(RABBIT_RIGHT_HAUNCH_POSE, &RABBIT_HAUNCH_CUBES),
        )],
    );
    let left_hind_leg = ModelPart::new(
        RABBIT_LEFT_HIND_LEG_POSE,
        Vec::new(),
        vec![(
            "haunch",
            ModelPart::leaf_colored(RABBIT_LEFT_HAUNCH_POSE, &RABBIT_HAUNCH_CUBES),
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

// `body_r1`: the 4×3×6 trunk.
pub(in crate::entity_models) const BABY_RABBIT_BODY_CUBES: [ModelCubeDesc; 1] =
    [cube([-2.0, -2.0, -3.0], [4.0, 3.0, 6.0], RABBIT_BROWN)];
// `tail_r1`: the 3×3×3 puff.
pub(in crate::entity_models) const BABY_RABBIT_TAIL_CUBES: [ModelCubeDesc; 1] = [cube(
    [-1.4, -2.0268, -1.0177],
    [3.0, 3.0, 3.0],
    RABBIT_BROWN,
)];
// `head`: the 5×4×4 skull.
pub(in crate::entity_models) const BABY_RABBIT_HEAD_CUBES: [ModelCubeDesc; 1] =
    [cube([-2.5, -3.0, -3.0], [5.0, 4.0, 4.0], RABBIT_BROWN)];
// The shared 2×4×1 ear (both ears reuse it).
pub(in crate::entity_models) const BABY_RABBIT_EAR_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.0, -3.5, -0.5], [2.0, 4.0, 1.0], RABBIT_BROWN)];
// The shared 1×3×1 front-leg box (both `_r1` legs reuse it).
pub(in crate::entity_models) const BABY_RABBIT_FRONT_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-0.5, -1.5, -0.5], [1.0, 3.0, 1.0], RABBIT_BROWN)];
// The shared 2×1×3 haunch box (both reuse it).
pub(in crate::entity_models) const BABY_RABBIT_HAUNCH_CUBES: [ModelCubeDesc; 1] =
    [cube([-2.0, -0.5, 0.0], [2.0, 1.0, 3.0], RABBIT_BROWN)];

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
            ModelPart::leaf_colored(BABY_RABBIT_TAIL_R1_POSE, &BABY_RABBIT_TAIL_CUBES),
        )],
    );
    let head = ModelPart::colored(
        BABY_RABBIT_HEAD_POSE,
        &BABY_RABBIT_HEAD_CUBES,
        vec![
            ModelPart::leaf_colored(BABY_RABBIT_RIGHT_EAR_POSE, &BABY_RABBIT_EAR_CUBES),
            ModelPart::leaf_colored(BABY_RABBIT_LEFT_EAR_POSE, &BABY_RABBIT_EAR_CUBES),
        ],
    );
    let left_front_leg = ModelPart::new(
        BABY_RABBIT_LEFT_FRONT_LEG_POSE,
        Vec::new(),
        vec![(
            "left_front_leg_r1",
            ModelPart::leaf_colored(BABY_RABBIT_FRONT_LEG_R1_POSE, &BABY_RABBIT_FRONT_LEG_CUBES),
        )],
    );
    let right_front_leg = ModelPart::new(
        BABY_RABBIT_RIGHT_FRONT_LEG_POSE,
        Vec::new(),
        vec![(
            "right_front_leg_r1",
            ModelPart::leaf_colored(BABY_RABBIT_FRONT_LEG_R1_POSE, &BABY_RABBIT_FRONT_LEG_CUBES),
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
                ModelPart::leaf_colored(BABY_RABBIT_BODY_R1_POSE, &BABY_RABBIT_BODY_CUBES),
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
            ModelPart::leaf_colored(BABY_RABBIT_LEFT_HAUNCH_POSE, &BABY_RABBIT_HAUNCH_CUBES),
        )],
    );
    let right_hind_leg = ModelPart::new(
        BABY_RABBIT_RIGHT_HIND_LEG_POSE,
        Vec::new(),
        vec![(
            "haunch",
            ModelPart::leaf_colored(BABY_RABBIT_RIGHT_HAUNCH_POSE, &BABY_RABBIT_HAUNCH_CUBES),
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
/// parts hang off a synthetic root, built from the baked colored geometry for the selected `baby`
/// layout. Colored-only: `setup_anim` turns the body-nested `head` (`child_mut("body").child_mut("head")`)
/// to the look angles (the hop / idle-head-tilt keyframes stay deferred).
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
