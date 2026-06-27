use super::{PartPose, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_PUFFERFISH_SMALL: &str =
    "minecraft:pufferfish_small#main";
pub(in crate::entity_models) const MODEL_LAYER_PUFFERFISH_MEDIUM: &str =
    "minecraft:pufferfish_medium#main";
pub(in crate::entity_models) const MODEL_LAYER_PUFFERFISH_BIG: &str =
    "minecraft:pufferfish_big#main";

// The pufferfish fallback paints its body and fins a sandy yellow.
pub(in crate::entity_models) const PUFFERFISH_YELLOW: [f32; 4] = [0.93, 0.80, 0.22, 1.0];

/// One pufferfish body-layer part: offset, rotation, cube min, cube size, and the
/// `texOffs(u, v)` UV origin. Every cube samples its full size and is never mirrored, so a
/// single descriptor row drives both the colored and textured geometry.
#[derive(Clone, Copy)]
pub(in crate::entity_models) struct PufferfishPart {
    pub offset: [f32; 3],
    pub rotation: [f32; 3],
    pub min: [f32; 3],
    pub size: [f32; 3],
    pub tex: [f32; 2],
}

impl PufferfishPart {
    pub(in crate::entity_models) fn pose(&self) -> PartPose {
        PartPose {
            offset: self.offset,
            rotation: self.rotation,
        }
    }

    /// The unified [`ModelCube`] for this part: a single never-mirrored box whose colored tint is the
    /// sandy [`PUFFERFISH_YELLOW`] and whose `uv_size` equals `size` (vanilla `CubeDeformation.NONE`).
    pub(in crate::entity_models) fn model_cube(&self) -> ModelCube {
        ModelCube::new(
            self.min,
            self.size,
            PUFFERFISH_YELLOW,
            self.size,
            self.tex,
            false,
        )
    }
}

const fn part(
    offset: [f32; 3],
    rotation: [f32; 3],
    min: [f32; 3],
    size: [f32; 3],
    tex: [f32; 2],
) -> PufferfishPart {
    PufferfishPart {
        offset,
        rotation,
        min,
        size,
        tex,
    }
}

use std::f32::consts::FRAC_PI_4;
const NEG_FRAC_PI_4: f32 = -FRAC_PI_4;

// Vanilla 26.1 PufferfishSmallModel.createBodyLayer (texture 32x32). The two pectoral fins
// (`right_fin`/`left_fin`, indices 4/5) wiggle on `ageInTicks`.
pub(in crate::entity_models) const PUFFERFISH_SMALL_PARTS: [PufferfishPart; 6] = [
    part(
        [0.0, 23.0, 0.0],
        [0.0; 3],
        [-1.5, -2.0, -1.5],
        [3.0, 2.0, 3.0],
        [0.0, 27.0],
    ),
    part(
        [0.0, 20.0, 0.0],
        [0.0; 3],
        [-1.5, 0.0, -1.5],
        [1.0, 1.0, 1.0],
        [24.0, 6.0],
    ),
    part(
        [0.0, 20.0, 0.0],
        [0.0; 3],
        [0.5, 0.0, -1.5],
        [1.0, 1.0, 1.0],
        [28.0, 6.0],
    ),
    part(
        [0.0, 22.0, 1.5],
        [0.0; 3],
        [-1.5, 0.0, 0.0],
        [3.0, 0.0, 3.0],
        [-3.0, 0.0],
    ),
    part(
        [-1.5, 22.0, -1.5],
        [0.0; 3],
        [-1.0, 0.0, 0.0],
        [1.0, 0.0, 2.0],
        [25.0, 0.0],
    ),
    part(
        [1.5, 22.0, -1.5],
        [0.0; 3],
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 2.0],
        [25.0, 0.0],
    ),
];

/// Vanilla `PufferfishSmallModel.createBodyLayer` child names, in declaration order. The two
/// pectoral fins (`right_fin`/`left_fin`) wiggle on `ageInTicks`; `child_mut` resolves them by name.
pub(in crate::entity_models) const PUFFERFISH_SMALL_NAMES: [&str; 6] = [
    "body",
    "right_eye",
    "left_eye",
    "back_fin",
    "right_fin",
    "left_fin",
];
pub(in crate::entity_models) const PUFFERFISH_SMALL_FIN_NAMES: [&str; 2] =
    ["right_fin", "left_fin"];

// Vanilla 26.1 PufferfishMidModel.createBodyLayer (texture 32x32). The two blue fins
// (`right_blue_fin`/`left_blue_fin`) wiggle on `ageInTicks`.
pub(in crate::entity_models) const PUFFERFISH_MID_PARTS: [PufferfishPart; 11] = [
    part(
        [0.0, 22.0, 0.0],
        [0.0; 3],
        [-2.5, -5.0, -2.5],
        [5.0, 5.0, 5.0],
        [12.0, 22.0],
    ),
    part(
        [-2.5, 18.0, -1.5],
        [0.0; 3],
        [-2.0, 0.0, 0.0],
        [2.0, 0.0, 2.0],
        [24.0, 0.0],
    ),
    part(
        [2.5, 18.0, -1.5],
        [0.0; 3],
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 2.0],
        [24.0, 3.0],
    ),
    part(
        [0.0, 17.0, -2.5],
        [FRAC_PI_4, 0.0, 0.0],
        [-2.5, -1.0, 0.0],
        [5.0, 1.0, 0.0],
        [19.0, 17.0],
    ),
    part(
        [0.0, 17.0, 2.5],
        [NEG_FRAC_PI_4, 0.0, 0.0],
        [-2.5, -1.0, 0.0],
        [5.0, 1.0, 0.0],
        [11.0, 17.0],
    ),
    part(
        [-2.5, 22.0, -2.5],
        [0.0, NEG_FRAC_PI_4, 0.0],
        [-1.0, -5.0, 0.0],
        [1.0, 5.0, 0.0],
        [5.0, 17.0],
    ),
    part(
        [-2.5, 22.0, 2.5],
        [0.0, FRAC_PI_4, 0.0],
        [-1.0, -5.0, 0.0],
        [1.0, 5.0, 0.0],
        [9.0, 17.0],
    ),
    part(
        [2.5, 22.0, 2.5],
        [0.0, NEG_FRAC_PI_4, 0.0],
        [0.0, -5.0, 0.0],
        [1.0, 5.0, 0.0],
        [1.0, 17.0],
    ),
    part(
        [2.5, 22.0, -2.5],
        [0.0, FRAC_PI_4, 0.0],
        [0.0, -5.0, 0.0],
        [1.0, 5.0, 0.0],
        [1.0, 17.0],
    ),
    part(
        [-2.5, 22.0, 2.5],
        [FRAC_PI_4, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [5.0, 1.0, 0.0],
        [18.0, 20.0],
    ),
    part(
        [0.0, 22.0, -2.5],
        [NEG_FRAC_PI_4, 0.0, 0.0],
        [-2.5, 0.0, 0.0],
        [5.0, 1.0, 1.0],
        [17.0, 19.0],
    ),
];

/// Vanilla `PufferfishMidModel.createBodyLayer` child names, in declaration order. The two blue fins
/// (`right_blue_fin`/`left_blue_fin`) wiggle on `ageInTicks`; `child_mut` resolves them by name.
pub(in crate::entity_models) const PUFFERFISH_MID_NAMES: [&str; 11] = [
    "body",
    "right_blue_fin",
    "left_blue_fin",
    "top_front_fin",
    "top_back_fin",
    "right_front_fin",
    "right_back_fin",
    "left_back_fin",
    "left_front_fin",
    "bottom_back_fin",
    "bottom_front_fin",
];
pub(in crate::entity_models) const PUFFERFISH_MID_FIN_NAMES: [&str; 2] =
    ["right_blue_fin", "left_blue_fin"];

// Vanilla 26.1 PufferfishBigModel.createBodyLayer (texture 32x32). The two blue fins
// (`right_blue_fin`/`left_blue_fin`) wiggle on `ageInTicks`.
pub(in crate::entity_models) const PUFFERFISH_BIG_PARTS: [PufferfishPart; 13] = [
    part(
        [0.0, 22.0, 0.0],
        [0.0; 3],
        [-4.0, -8.0, -4.0],
        [8.0, 8.0, 8.0],
        [0.0, 0.0],
    ),
    part(
        [-4.0, 15.0, -2.0],
        [0.0; 3],
        [-2.0, 0.0, -1.0],
        [2.0, 1.0, 2.0],
        [24.0, 0.0],
    ),
    part(
        [4.0, 15.0, -2.0],
        [0.0; 3],
        [0.0, 0.0, -1.0],
        [2.0, 1.0, 2.0],
        [24.0, 3.0],
    ),
    part(
        [0.0, 14.0, -4.0],
        [FRAC_PI_4, 0.0, 0.0],
        [-4.0, -1.0, 0.0],
        [8.0, 1.0, 0.0],
        [15.0, 17.0],
    ),
    part(
        [0.0, 14.0, 0.0],
        [0.0; 3],
        [-4.0, -1.0, 0.0],
        [8.0, 1.0, 1.0],
        [14.0, 16.0],
    ),
    part(
        [0.0, 14.0, 4.0],
        [NEG_FRAC_PI_4, 0.0, 0.0],
        [-4.0, -1.0, 0.0],
        [8.0, 1.0, 0.0],
        [23.0, 18.0],
    ),
    part(
        [-4.0, 22.0, -4.0],
        [0.0, NEG_FRAC_PI_4, 0.0],
        [-1.0, -8.0, 0.0],
        [1.0, 8.0, 0.0],
        [5.0, 17.0],
    ),
    part(
        [4.0, 22.0, -4.0],
        [0.0, FRAC_PI_4, 0.0],
        [0.0, -8.0, 0.0],
        [1.0, 8.0, 0.0],
        [1.0, 17.0],
    ),
    part(
        [0.0, 22.0, -4.0],
        [NEG_FRAC_PI_4, 0.0, 0.0],
        [-4.0, 0.0, 0.0],
        [8.0, 1.0, 0.0],
        [15.0, 20.0],
    ),
    part(
        [0.0, 22.0, 0.0],
        [0.0; 3],
        [-4.0, 0.0, 0.0],
        [8.0, 1.0, 0.0],
        [15.0, 20.0],
    ),
    part(
        [0.0, 22.0, 4.0],
        [FRAC_PI_4, 0.0, 0.0],
        [-4.0, 0.0, 0.0],
        [8.0, 1.0, 0.0],
        [15.0, 20.0],
    ),
    part(
        [-4.0, 22.0, 4.0],
        [0.0, FRAC_PI_4, 0.0],
        [-1.0, -8.0, 0.0],
        [1.0, 8.0, 0.0],
        [9.0, 17.0],
    ),
    part(
        [4.0, 22.0, 4.0],
        [0.0, NEG_FRAC_PI_4, 0.0],
        [0.0, -8.0, 0.0],
        [1.0, 8.0, 0.0],
        [9.0, 17.0],
    ),
];

/// Vanilla `PufferfishBigModel.createBodyLayer` child names, in declaration order. The two blue fins
/// (`right_blue_fin`/`left_blue_fin`) wiggle on `ageInTicks`; `child_mut` resolves them by name.
pub(in crate::entity_models) const PUFFERFISH_BIG_NAMES: [&str; 13] = [
    "body",
    "right_blue_fin",
    "left_blue_fin",
    "top_front_fin",
    "top_middle_fin",
    "top_back_fin",
    "right_front_fin",
    "left_front_fin",
    "bottom_front_fin",
    "bottom_middle_fin",
    "bottom_back_fin",
    "right_back_fin",
    "left_back_fin",
];
pub(in crate::entity_models) const PUFFERFISH_BIG_FIN_NAMES: [&str; 2] =
    ["right_blue_fin", "left_blue_fin"];

/// Returns the body-layer parts, their vanilla child names, and the two `ageInTicks`-wiggling fin
/// names for a puff state (`0` small, `1` mid, `>=2` big — matching `PufferfishRenderer.submit`).
pub(in crate::entity_models) fn pufferfish_parts(
    puff_state: i32,
) -> (
    &'static [PufferfishPart],
    &'static [&'static str],
    [&'static str; 2],
) {
    match puff_state {
        0 => (
            &PUFFERFISH_SMALL_PARTS,
            &PUFFERFISH_SMALL_NAMES,
            PUFFERFISH_SMALL_FIN_NAMES,
        ),
        1 => (
            &PUFFERFISH_MID_PARTS,
            &PUFFERFISH_MID_NAMES,
            PUFFERFISH_MID_FIN_NAMES,
        ),
        _ => (
            &PUFFERFISH_BIG_PARTS,
            &PUFFERFISH_BIG_NAMES,
            PUFFERFISH_BIG_FIN_NAMES,
        ),
    }
}

/// Vanilla pufferfish fin wiggle for the right fin: `right.zRot = -0.2 + 0.4 *
/// sin(ageInTicks * 0.2)`. The left fin is the negation. Shared verbatim by
/// `PufferfishSmallModel`/`MidModel`/`BigModel.setupAnim` (the small model wiggles its
/// pectoral fins, the mid/big their blue fins), set absolutely over the zeroed rest pose.
pub(in crate::entity_models) fn pufferfish_right_fin_z_rot(age_in_ticks: f32) -> f32 {
    -0.2 + 0.4 * (age_in_ticks * 0.2).sin()
}

/// Applies the fin wiggle `zRot` to a fin part pose, preserving the offset and the zeroed
/// `xRot`/`yRot`.
pub(in crate::entity_models) fn pufferfish_fin_pose(base: PartPose, z_rot: f32) -> PartPose {
    PartPose {
        offset: base.offset,
        rotation: [base.rotation[0], base.rotation[1], z_rot],
    }
}

/// Mutable pufferfish model, mirroring vanilla `PufferfishSmallModel`/`MidModel`/`BigModel`. The puff
/// state picks one of the three flat part lists ([`pufferfish_parts`]); each part hangs off a synthetic
/// root as a named child carrying its single cube (both render paths' data), so one tree drives the
/// colored fallback and the cutout textured layer. `setup_anim` wiggles the two `ageInTicks`-driven
/// fins by name; the body bob lives in the pufferfish root transform.
pub(in crate::entity_models) struct PufferfishModel {
    root: ModelPart,
    fins: [&'static str; 2],
}

impl PufferfishModel {
    pub(in crate::entity_models) fn new(puff_state: i32) -> Self {
        let (parts, names, fins) = pufferfish_parts(puff_state);
        let children = parts
            .iter()
            .zip(names.iter())
            .map(|(part, &name)| (name, ModelPart::leaf(part.pose(), vec![part.model_cube()])))
            .collect();
        Self {
            root: ModelPart::new(PART_POSE_ZERO, Vec::new(), children),
            fins,
        }
    }
}

impl EntityModel for PufferfishModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        // Vanilla `setupAnim` wiggles the two fins on `ageInTicks`: the right fin takes `zRot`, the
        // left its negation, set absolutely over the zeroed rest `zRot` (offset / `xRot` / `yRot`
        // preserved).
        let fin_z = pufferfish_right_fin_z_rot(instance.render_state.age_in_ticks);
        let right = self.root.child_mut(self.fins[0]);
        right.pose = pufferfish_fin_pose(right.pose, fin_z);
        let left = self.root.child_mut(self.fins[1]);
        left.pose = pufferfish_fin_pose(left.pose, -fin_z);
    }
}
