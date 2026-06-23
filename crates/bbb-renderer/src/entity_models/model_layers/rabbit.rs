use super::{
    apply_head_look, bind_part as part, bind_part_rot as rpart, model_cube as cube, ModelCubeDesc,
    ModelPartDesc, RABBIT_BROWN,
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
// renders one brown tint. Rabbit uses a plain `MobRenderer`/`LivingEntityRenderer.setupRotations`.

// `body`: the 8×6×10 torso.
const RABBIT_BODY_CUBES: [ModelCubeDesc; 1] =
    [cube([-4.0, -6.0, -9.0], [8.0, 6.0, 10.0], RABBIT_BROWN)];

// `tail`: the 4×4×4 puff.
const RABBIT_TAIL_CUBES: [ModelCubeDesc; 1] = [cube(
    [-2.0, -3.0084, -1.0125],
    [4.0, 4.0, 4.0],
    RABBIT_BROWN,
)];

// `head`: the 5×5×5 skull.
const RABBIT_HEAD_CUBES: [ModelCubeDesc; 1] =
    [cube([-2.5, -3.0, -4.0], [5.0, 5.0, 5.0], RABBIT_BROWN)];

// The shared 2×5×1 ear (both ears reuse it, differing only in pivot X sign).
const RABBIT_EAR_CUBES: [ModelCubeDesc; 1] = [cube(
    [-1.0, -4.2929, -0.1213],
    [2.0, 5.0, 1.0],
    RABBIT_BROWN,
)];

// The two 2×4×2 front legs (the right one's box is nudged on X, matching vanilla).
const RABBIT_RIGHT_FRONT_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-0.9, -1.0, -0.9], [2.0, 4.0, 2.0], RABBIT_BROWN)];
const RABBIT_LEFT_FRONT_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.0, -1.0, -1.0], [2.0, 4.0, 2.0], RABBIT_BROWN)];

// The shared 2×1×6 haunch (both reuse it, differing only in the haunch yaw sign).
const RABBIT_HAUNCH_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.0, 0.0, -5.0], [2.0, 1.0, 6.0], RABBIT_BROWN)];

// `head` children: the left and right ears.
const RABBIT_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    part([1.5, -3.7071, -0.8787], &RABBIT_EAR_CUBES, &[]),
    part([-1.5, -3.7071, -0.8787], &RABBIT_EAR_CUBES, &[]),
];

// `frontlegs` (a cubeless pivot) children: the right and left front legs.
const RABBIT_FRONTLEGS_CHILDREN: [ModelPartDesc; 2] = [
    rpart(
        [-2.0, 1.9239, 0.3827],
        [0.3927, 0.0, 0.0],
        &RABBIT_RIGHT_FRONT_LEG_CUBES,
        &[],
    ),
    rpart(
        [2.0, 1.9239, 0.4827],
        [0.3927, 0.0, 0.0],
        &RABBIT_LEFT_FRONT_LEG_CUBES,
        &[],
    ),
];

// `body` children: the tail, the head (with ears), and the cubeless `frontlegs` pivot.
const RABBIT_BODY_CHILDREN: [ModelPartDesc; 3] = [
    part([0.0, -4.9916, 0.0125], &RABBIT_TAIL_CUBES, &[]),
    rpart(
        [0.0, -5.2929, -8.1213],
        [0.3927, 0.0, 0.0],
        &RABBIT_HEAD_CUBES,
        &RABBIT_HEAD_CHILDREN,
    ),
    part([0.0, -1.5349, -6.3108], &[], &RABBIT_FRONTLEGS_CHILDREN),
];

// Each hind leg (a cubeless pivot) parents its haunch (the haunch carries the only cube).
const RABBIT_RIGHT_HIND_LEG_CHILDREN: [ModelPartDesc; 1] = [rpart(
    [0.0, -0.5, 0.0],
    [0.0, 0.3927, 0.0],
    &RABBIT_HAUNCH_CUBES,
    &[],
)];
const RABBIT_LEFT_HIND_LEG_CHILDREN: [ModelPartDesc; 1] = [rpart(
    [0.0, -0.5, 0.0],
    [0.0, -0.3927, 0.0],
    &RABBIT_HAUNCH_CUBES,
    &[],
)];

// `backlegs` (a cubeless pivot) children: the right and left hind legs.
const RABBIT_BACKLEGS_CHILDREN: [ModelPartDesc; 2] = [
    part([-3.0, 0.5, 0.0], &[], &RABBIT_RIGHT_HIND_LEG_CHILDREN),
    part([3.0, 0.5, 0.0], &[], &RABBIT_LEFT_HIND_LEG_CHILDREN),
];

/// Vanilla `AdultRabbitModel.createBodyLayer` rest-pose hierarchy: the `body` (pitched `-0.3927`)
/// and the cubeless `backlegs` pivot. Nine cubes (the `frontlegs`, `backlegs`, and the two hind-leg
/// parts are cubeless pivots).
pub(in crate::entity_models) const ADULT_RABBIT_PARTS: [ModelPartDesc; 2] = [
    rpart(
        [0.0, 23.0, 4.0],
        [-0.3927, 0.0, 0.0],
        &RABBIT_BODY_CUBES,
        &RABBIT_BODY_CHILDREN,
    ),
    part([0.0, 23.0, 4.0], &[], &RABBIT_BACKLEGS_CHILDREN),
];

/// The `head` is `body`'s second child (tail `0`, head `1`, frontlegs `2`); `RabbitModel.setupAnim`
/// sets its `xRot/yRot` from the look angles.
const RABBIT_HEAD_BODY_CHILD_INDEX: usize = 1;

/// Mutable rabbit model, mirroring vanilla `AdultRabbitModel`. The two root parts hang off a synthetic
/// root, each built from the baked [`ADULT_RABBIT_PARTS`] geometry. Colored-only (adult): `setup_anim`
/// turns the body-nested head to the look angles (the hop / idle-head-tilt keyframes and the baby
/// layer stay deferred).
pub(in crate::entity_models) struct RabbitModel {
    root: ModelPart,
}

impl RabbitModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::root_from_colored_descs(&ADULT_RABBIT_PARTS),
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
        // that overwrites the head's baked `0.3927` pitch) whenever the idle-head-tilt animation is
        // not playing — and bbb never projects that `AnimationState`, so the look applies every frame.
        let head = self
            .root
            .child_at_mut(0)
            .child_at_mut(RABBIT_HEAD_BODY_CHILD_INDEX);
        apply_head_look(
            head,
            instance.render_state.head_yaw,
            instance.render_state.head_pitch,
        );
    }
}
