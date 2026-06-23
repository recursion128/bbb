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

// Vanilla 26.1 `BabyFelineModel.createBodyMesh` (atlas 32×32), shared unscaled by the baby cat
// (`ModelLayers.CAT_BABY`) and the baby ocelot (`ModelLayers.OCELOT_BABY`) — neither gets the adult
// cat's 0.8 transform. A flatter, all-upright layout: no pitched body, the head carries its ears and
// nose, and `tail2` is an empty pivot (no cube). The base `BabyFelineModel.setupAnim` still sets
// `head.xRot/yRot` from the look and `tail2.xRot = 1.7278761`, but the latter is invisible here (the
// cubeless lower tail has no geometry), so the baby's only reproduced pose is the head look.

// Baby `head` cubes (offset 0,20,-3.125): the 5×4×4 skull, two 1×1×2 ears, and a 3×2×1 nose.
const BABY_FELINE_HEAD_CUBES: [ModelCubeDesc; 4] = [
    cube([-2.5, -3.0, -2.875], [5.0, 4.0, 4.0], FELINE_TAN),
    cube([-2.0, -4.0, -0.875], [1.0, 1.0, 2.0], FELINE_TAN),
    cube([1.0, -4.0, -0.875], [1.0, 1.0, 2.0], FELINE_TAN),
    cube([-1.5, -1.0, -3.875], [3.0, 2.0, 1.0], FELINE_TAN),
];

// Baby `body`: the 4×3×7 trunk (upright, not pitched).
const BABY_FELINE_BODY_CUBES: [ModelCubeDesc; 1] =
    [cube([-2.0, -1.5, -3.5], [4.0, 3.0, 7.0], FELINE_TAN)];

// Baby legs: one shared 1×2×2 box for all four.
const BABY_FELINE_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-0.5, 0.0, -1.0], [1.0, 2.0, 2.0], FELINE_TAN)];

// Baby `tail1`: the single 1×1×5 tail segment (`tail2` below it is cubeless).
const BABY_FELINE_TAIL1_CUBES: [ModelCubeDesc; 1] =
    [cube([-0.5, -0.107, 0.0849], [1.0, 1.0, 5.0], FELINE_TAN)];

/// Vanilla `BabyFelineModel.createBodyMesh` rest-pose hierarchy (`addOrReplaceChild` order): `head`
/// (0), the left-front / right-front / left-hind legs (1..=3), `body` (4), `right_hind_leg` (5),
/// `tail1` (6, pitched `-0.567232`), and the cubeless `tail2` (7). Ten cubes.
pub(in crate::entity_models) const BABY_FELINE_PARTS: [ModelPartDesc; 8] = [
    part([0.0, 20.0, -3.125], &BABY_FELINE_HEAD_CUBES, &[]),
    part([1.0, 22.0, -1.5], &BABY_FELINE_LEG_CUBES, &[]),
    part([-1.0, 22.0, -1.5], &BABY_FELINE_LEG_CUBES, &[]),
    part([1.0, 22.0, 2.5], &BABY_FELINE_LEG_CUBES, &[]),
    part([0.0, 20.5, 0.5], &BABY_FELINE_BODY_CUBES, &[]),
    part([-1.0, 22.0, 2.5], &BABY_FELINE_LEG_CUBES, &[]),
    rpart(
        [0.0, 19.107, 3.9151],
        [-0.567232, 0.0, 0.0],
        &BABY_FELINE_TAIL1_CUBES,
        &[],
    ),
    part([0.0, 0.0, 0.0], &[], &[]),
];

/// The `head` is the first root part in both layouts; the adult's `tail2` is part 3 (the baby's
/// `tail2` is cubeless, so its droop is skipped).
const FELINE_HEAD_PART_INDEX: usize = 0;
const FELINE_TAIL2_PART_INDEX: usize = 3;

/// Picks the adult or baby feline geometry. Both share the head-look pose; only the adult has a
/// cubed `tail2` to drop to its standing rest pitch.
const fn feline_parts(baby: bool) -> &'static [ModelPartDesc] {
    if baby {
        &BABY_FELINE_PARTS
    } else {
        &FELINE_PARTS
    }
}

/// Vanilla `AdultFelineModel.setupAnim` standing tail droop: while not sitting it sets
/// `tail2.xRot = 1.7278761` (`= π·0.55`), the base the walk wobble adds onto. With the wobble deferred
/// (walk speed dependent), this is the resting `tail2` pitch — a real change from the `0` bind rotation.
const FELINE_TAIL2_REST_X_ROT: f32 = 1.7278761;

/// Mutable feline model, mirroring vanilla `AdultFelineModel` / `BabyFelineModel` (ocelot and cat
/// share each). The root parts hang off a synthetic root, built from the baked [`feline_parts`]
/// geometry; the adult cat's 0.8 scale lives in the root transform ([`FELINE_CAT_SCALE`], applied by
/// the runtime — the babies are unscaled). Colored-only: `setup_anim` runs the head look
/// ([`apply_head_look`]) and, for the adult, drops the lower tail to its standing rest pitch; the walk
/// swing/wobble and every feline pose stay deferred.
pub(in crate::entity_models) struct FelineModel {
    root: ModelPart,
    baby: bool,
}

impl FelineModel {
    pub(in crate::entity_models) fn new(baby: bool) -> Self {
        Self {
            root: ModelPart::root_from_colored_descs(feline_parts(baby)),
            baby,
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
        // adds onto it); with the wobble deferred this is the resting lower-tail droop. The baby's
        // `tail2` is cubeless, so vanilla's identical assignment there is invisible — we skip it.
        if !self.baby {
            self.root
                .child_at_mut(FELINE_TAIL2_PART_INDEX)
                .pose
                .rotation[0] = FELINE_TAIL2_REST_X_ROT;
        }
    }
}
