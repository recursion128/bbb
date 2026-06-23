use super::{
    apply_head_look, bind_part as part, bind_part_rot as rpart, model_cube as cube, ModelCubeDesc,
    ModelPartDesc, FELINE_TAN,
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
const FELINE_HEAD_CUBES: [ModelCubeDesc; 4] = [
    cube([-2.5, -2.0, -3.0], [5.0, 4.0, 5.0], FELINE_TAN),
    cube([-1.5, -0.001, -4.0], [3.0, 2.0, 2.0], FELINE_TAN),
    cube([-2.0, -3.0, 0.0], [1.0, 1.0, 2.0], FELINE_TAN),
    cube([1.0, -3.0, 0.0], [1.0, 1.0, 2.0], FELINE_TAN),
];

// `body`: the 4×16×6 trunk (pitched onto its belly).
const FELINE_BODY_CUBES: [ModelCubeDesc; 1] =
    [cube([-2.0, 3.0, -8.0], [4.0, 16.0, 6.0], FELINE_TAN)];

// `tail1`: the upper 1×8×1 tail segment.
const FELINE_TAIL1_CUBES: [ModelCubeDesc; 1] =
    [cube([-0.5, 0.0, 0.0], [1.0, 8.0, 1.0], FELINE_TAN)];

// `tail2`: the lower 1×8×1 tail segment, deflated by the vanilla `CubeDeformation(-0.02)` (min += 0.02,
// size -= 0.04).
const FELINE_TAIL2_CUBES: [ModelCubeDesc; 1] =
    [cube([-0.48, 0.02, 0.02], [0.96, 7.96, 0.96], FELINE_TAN)];

// The two 2×6×2 hind legs (shared box), and the two 2×10×2 front legs (shared box).
const FELINE_HIND_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.0, 0.0, 1.0], [2.0, 6.0, 2.0], FELINE_TAN)];
const FELINE_FRONT_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.0, 0.0, 0.0], [2.0, 10.0, 2.0], FELINE_TAN)];

/// Vanilla `AdultFelineModel.createBodyMesh` rest-pose hierarchy (`addOrReplaceChild` order): `head`
/// (0), `body` (1, pitched `π/2`), `tail1` (2, pitched `0.9`), `tail2` (3), then the left-hind /
/// right-hind / left-front / right-front legs (4..=7). Eleven cubes.
pub(in crate::entity_models) const FELINE_PARTS: [ModelPartDesc; 8] = [
    part([0.0, 15.0, -9.0], &FELINE_HEAD_CUBES, &[]),
    rpart(
        [0.0, 12.0, -10.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        &FELINE_BODY_CUBES,
        &[],
    ),
    rpart([0.0, 15.0, 8.0], [0.9, 0.0, 0.0], &FELINE_TAIL1_CUBES, &[]),
    part([0.0, 20.0, 14.0], &FELINE_TAIL2_CUBES, &[]),
    part([1.1, 18.0, 5.0], &FELINE_HIND_LEG_CUBES, &[]),
    part([-1.1, 18.0, 5.0], &FELINE_HIND_LEG_CUBES, &[]),
    part([1.2, 14.1, -5.0], &FELINE_FRONT_LEG_CUBES, &[]),
    part([-1.2, 14.1, -5.0], &FELINE_FRONT_LEG_CUBES, &[]),
];

/// The `head` is the first root part; `tail2` is part 3.
const FELINE_HEAD_PART_INDEX: usize = 0;
const FELINE_TAIL2_PART_INDEX: usize = 3;

/// Vanilla `AdultFelineModel.setupAnim` standing tail droop: while not sitting it sets
/// `tail2.xRot = 1.7278761` (`= π·0.55`), the base the walk wobble adds onto. With the wobble deferred
/// (walk speed dependent), this is the resting `tail2` pitch — a real change from the `0` bind rotation.
const FELINE_TAIL2_REST_X_ROT: f32 = 1.7278761;

/// Mutable feline model, mirroring vanilla `AdultFelineModel` (ocelot and cat share it). The eight root
/// parts hang off a synthetic root, built from the baked [`FELINE_PARTS`] geometry; the cat's 0.8 scale
/// lives in the root transform ([`FELINE_CAT_SCALE`]). Colored-only: `setup_anim` runs the head look
/// ([`apply_head_look`]) and drops the lower tail to its standing rest pitch; the walk swing/wobble and
/// every feline pose stay deferred.
pub(in crate::entity_models) struct FelineModel {
    root: ModelPart,
}

impl FelineModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::root_from_colored_descs(&FELINE_PARTS),
        }
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
            self.root.child_at_mut(FELINE_HEAD_PART_INDEX),
            render_state.head_yaw,
            render_state.head_pitch,
        );
        // Vanilla's not-sitting branch sets `tail2.xRot = 1.7278761` unconditionally (the walk wobble
        // adds onto it); with the wobble deferred this is the resting lower-tail droop.
        self.root
            .child_at_mut(FELINE_TAIL2_PART_INDEX)
            .pose
            .rotation[0] = FELINE_TAIL2_REST_X_ROT;
    }
}
