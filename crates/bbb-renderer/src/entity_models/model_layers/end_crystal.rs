use super::{
    bind_part as part, model_cube as cube, ModelCubeDesc, ModelPartDesc, END_CRYSTAL_BASE,
    END_CRYSTAL_CORE, END_CRYSTAL_GLASS,
};

// Vanilla 26.1 `EndCrystalModel.createBodyLayer` (atlas 64×32). The mesh root holds the base slab
// (at ZERO) and the nested glass stack: `outer_glass` at `offset(0, 24, 0)` parents `inner_glass`
// (`PartPose.ZERO.withScale(0.875)`), which parents the core `cube` (`PartPose.ZERO.withScale(
// 0.765625)`). All three glass boxes are the same centered 8×8×8 cube; the per-part `withScale` is
// cumulative through the hierarchy (`inner_glass` renders at 0.875×, the core at 0.875 · 0.765625 =
// 0.669921875×). Since every glass part shares the same `(0, 24, 0)` centre and the rest pose has no
// rotation, the uniform scales are baked into the centred cube dimensions (a scaled centred cube is
// a smaller centred cube), exactly reproducing the static pose. The `EndCrystalModel.setupAnim`
// `base.visible = showsBottom` toggle IS reproduced (the base slab — `END_CRYSTAL_PARTS[0]` — is
// gated on the synced `showsBottom` state); the `outer_glass`/`inner_glass`/`cube` diagonal spin
// (`Axis.YP.rotationDegrees(ageInTicks · 3) · ...`) and the `getY` vertical bob remain deferred.
// `EndCrystalRenderer` is a plain `EntityRenderer` with
// only the `scale(2.0)` + `translate(0, -0.5, 0)` transform (no `LivingEntityRenderer` flip). The
// texture-backed path is deferred, so the colored debug path renders the magenta glass, the bright
// core, and the dark base with three tints.

// `base`: the 12×4×12 bedrock slab at the model origin.
const END_CRYSTAL_BASE_CUBES: [ModelCubeDesc; 1] =
    [cube([-6.0, 0.0, -6.0], [12.0, 4.0, 12.0], END_CRYSTAL_BASE)];

// `outer_glass`: the unscaled 8×8×8 glass cube.
const END_CRYSTAL_OUTER_GLASS_CUBES: [ModelCubeDesc; 1] =
    [cube([-4.0, -4.0, -4.0], [8.0, 8.0, 8.0], END_CRYSTAL_GLASS)];

// `inner_glass`: the 8×8×8 cube at `withScale(0.875)` → a centred 7×7×7 box.
const END_CRYSTAL_INNER_GLASS_CUBES: [ModelCubeDesc; 1] =
    [cube([-3.5, -3.5, -3.5], [7.0, 7.0, 7.0], END_CRYSTAL_GLASS)];

// `cube`: the core 8×8×8 cube at the cumulative `0.875 · 0.765625 = 0.669921875` scale → a centred
// 5.359375×5.359375×5.359375 box.
const END_CRYSTAL_CORE_CUBES: [ModelCubeDesc; 1] = [cube(
    [-2.6796875, -2.6796875, -2.6796875],
    [5.359375, 5.359375, 5.359375],
    END_CRYSTAL_CORE,
)];

pub(in crate::entity_models) const END_CRYSTAL_PARTS: [ModelPartDesc; 4] = [
    part([0.0, 0.0, 0.0], &END_CRYSTAL_BASE_CUBES, &[]),
    part([0.0, 24.0, 0.0], &END_CRYSTAL_OUTER_GLASS_CUBES, &[]),
    part([0.0, 24.0, 0.0], &END_CRYSTAL_INNER_GLASS_CUBES, &[]),
    part([0.0, 24.0, 0.0], &END_CRYSTAL_CORE_CUBES, &[]),
];
