use super::{
    bind_part as part, model_cube as cube, ModelCubeDesc, ModelPartDesc, TexturedModelCubeDesc,
    TexturedModelPartDesc, END_CRYSTAL_BASE, END_CRYSTAL_CORE, END_CRYSTAL_GLASS,
};
use glam::{Quat, Vec3};

// Vanilla 26.1 `EndCrystalModel.createBodyLayer` (atlas 64×32). The mesh root holds the base slab
// (at ZERO) and the nested glass stack: `outer_glass` at `offset(0, 24, 0)` parents `inner_glass`
// (`PartPose.ZERO.withScale(0.875)`), which parents the core `cube` (`PartPose.ZERO.withScale(
// 0.765625)`). All three glass boxes are the same centered 8×8×8 cube; the per-part `withScale` is
// cumulative through the hierarchy (`inner_glass` renders at 0.875×, the core at 0.875 · 0.765625 =
// 0.669921875×). Since every glass part shares the same `(0, 24, 0)` centre and the rest pose has no
// rotation, the uniform scales are baked into the centred cube dimensions (a scaled centred cube is
// a smaller centred cube), reproducing the scales. The full `EndCrystalModel.setupAnim` is now
// reproduced: `base.visible = showsBottom` gates the base slab (`END_CRYSTAL_PARTS[0]`); the
// `outer_glass`/`inner_glass`/`cube` diagonal spin ([`end_crystal_glass_quaternions`], the π/3
// diagonal tilt composed with `Ry(ageInTicks·3°)`) is applied by hand-walking the nested glass
// stack; and the `getY` vertical bob ([`end_crystal_bob_y`]) lifts the whole glass stack off the
// projected `age_in_ticks`. `EndCrystalRenderer` is a plain `EntityRenderer` with only the
// `scale(2.0)` + `translate(0, -0.5, 0)` transform (no `LivingEntityRenderer` flip). The textured
// path binds `end_crystal.png` through the same hand-walked setup animation; the colored path remains
// as a missing-atlas fallback with separate glass/core/base tints.

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

const END_CRYSTAL_BASE_TEXTURED_CUBES: [TexturedModelCubeDesc; 1] = [TexturedModelCubeDesc {
    min: [-6.0, 0.0, -6.0],
    size: [12.0, 4.0, 12.0],
    uv_size: [12.0, 4.0, 12.0],
    tex: [0.0, 16.0],
    mirror: false,
}];

const END_CRYSTAL_OUTER_GLASS_TEXTURED_CUBES: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, -4.0, -4.0],
        size: [8.0, 8.0, 8.0],
        uv_size: [8.0, 8.0, 8.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

const END_CRYSTAL_INNER_GLASS_TEXTURED_CUBES: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.5, -3.5, -3.5],
        size: [7.0, 7.0, 7.0],
        uv_size: [8.0, 8.0, 8.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

const END_CRYSTAL_CORE_TEXTURED_CUBES: [TexturedModelCubeDesc; 1] = [TexturedModelCubeDesc {
    min: [-2.6796875, -2.6796875, -2.6796875],
    size: [5.359375, 5.359375, 5.359375],
    uv_size: [8.0, 8.0, 8.0],
    tex: [32.0, 0.0],
    mirror: false,
}];

pub(in crate::entity_models) const END_CRYSTAL_TEXTURED_PARTS: [TexturedModelPartDesc; 4] = [
    TexturedModelPartDesc {
        pose: super::PART_POSE_ZERO,
        cubes: &END_CRYSTAL_BASE_TEXTURED_CUBES,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: super::PartPose {
            offset: [0.0, 24.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &END_CRYSTAL_OUTER_GLASS_TEXTURED_CUBES,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: super::PartPose {
            offset: [0.0, 24.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &END_CRYSTAL_INNER_GLASS_TEXTURED_CUBES,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: super::PartPose {
            offset: [0.0, 24.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &END_CRYSTAL_CORE_TEXTURED_CUBES,
        children: &[],
    },
];

/// Vanilla `EndCrystalRenderer.getY(timeInTicks)`: the hover height that drives both the glass bob
/// and the beam offset. `hh = sin(t·0.2)/2 + 0.5; hh = (hh² + hh)·0.4; return hh − 1.4`.
pub(in crate::entity_models) fn end_crystal_get_y(age_in_ticks: f32) -> f32 {
    let hh = (age_in_ticks * 0.2).sin() / 2.0 + 0.5;
    let hh = (hh * hh + hh) * 0.4;
    hh - 1.4
}

/// Vanilla `EndCrystalModel.setupAnim`: `outerGlass.y += getY(age)·16 / 2`, so the whole glass stack
/// (outer/inner/core) bobs by `getY(age)·8` model-pixels. Negative (the crystal hovers above the
/// base) and applied to the shared `(0, 24, 0)` glass centre.
pub(in crate::entity_models) fn end_crystal_bob_y(age_in_ticks: f32) -> f32 {
    end_crystal_get_y(age_in_ticks) * 16.0 / 2.0
}

/// Vanilla `EndCrystalModel.setupAnim` glass spin, as the two local `rotateBy` quaternions. With
/// `TILT` = π/3 about the `(sin45, 0, sin45)` diagonal and `spin` = `Ry(age·3°)`:
/// - `outer_glass` rotates by `Ry(age·3°)·TILT` (vanilla `Axis.YP.rotationDegrees(speed).rotateAxis(π/3, …)`),
/// - `inner_glass` and `cube` each rotate by `TILT·Ry(age·3°)` (vanilla `setAngleAxis(π/3, …).rotateY(speed)`).
///
/// Returns `(outer, inner_and_core)`. The renderer nests them (`outer → inner → core`) so the inner
/// and core inherit the outer's rotation, exactly as vanilla's part hierarchy does. Even at age 0
/// the `TILT` leaves the crystal canted on the diagonal axis.
pub(in crate::entity_models) fn end_crystal_glass_quaternions(age_in_ticks: f32) -> (Quat, Quat) {
    use std::f32::consts::{FRAC_1_SQRT_2, FRAC_PI_3};
    let axis = Vec3::new(FRAC_1_SQRT_2, 0.0, FRAC_1_SQRT_2);
    let tilt = Quat::from_axis_angle(axis, FRAC_PI_3);
    let spin = Quat::from_rotation_y((age_in_ticks * 3.0).to_radians());
    (spin * tilt, tilt * spin)
}
