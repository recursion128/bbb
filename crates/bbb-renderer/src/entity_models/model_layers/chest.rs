use std::f32::consts::FRAC_PI_2;

use super::{PartPose, CHEST_WOOD, PART_POSE_ZERO};
use crate::entity_models::catalog::ChestModelHalf;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_CHEST: &str = "minecraft:chest#main";
pub(in crate::entity_models) const MODEL_LAYER_CHEST_LEFT: &str =
    "minecraft:double_chest_left#main";
pub(in crate::entity_models) const MODEL_LAYER_CHEST_RIGHT: &str =
    "minecraft:double_chest_right#main";

const fn chest_cube(min: [f32; 3], size: [f32; 3], tex: [f32; 2]) -> ModelCube {
    ModelCube::new(min, size, CHEST_WOOD, size, tex, false)
}

// Vanilla 26.1 `ChestModel.createSingleBodyLayer` (atlas 64×64): `bottom` at ZERO with a 14×10×14
// box at texOffs(0, 19); `lid` and `lock` both pivot at offset(0, 9, 1) — the hinge line along the
// chest's back edge — with a 14×5×14 lid box at texOffs(0, 0) and a 2×4×1 lock box at texOffs(0, 0).
// Unlike entity models the chest mesh is authored Y-up in block-local space (the renderer applies no
// `scale(-1, -1, 1)` flip; `ChestRenderer.submit` only rotates about the block centre for `facing`).
pub(in crate::entity_models) const CHEST_SINGLE_BOTTOM_CUBE: ModelCube =
    chest_cube([1.0, 0.0, 1.0], [14.0, 10.0, 14.0], [0.0, 19.0]);
pub(in crate::entity_models) const CHEST_SINGLE_LID_CUBE: ModelCube =
    chest_cube([1.0, 0.0, 0.0], [14.0, 5.0, 14.0], [0.0, 0.0]);
pub(in crate::entity_models) const CHEST_SINGLE_LOCK_CUBE: ModelCube =
    chest_cube([7.0, -2.0, 14.0], [2.0, 4.0, 1.0], [0.0, 0.0]);

// Vanilla `ChestModel.createDoubleBodyRightLayer`: the 15-wide right half spans x = 1..16 toward the
// seam. (The vanilla mesh culls the seam-facing EAST faces via `Util.allOfEnumExcept(Direction.EAST)`;
// bbb keeps emitting the seam quads — they sit enclosed inside the joined double-chest volume and
// are never visible, so the chest mesh predates and does not use the `with_visible_faces` mask the
// bed introduced.)
pub(in crate::entity_models) const CHEST_RIGHT_BOTTOM_CUBE: ModelCube =
    chest_cube([1.0, 0.0, 1.0], [15.0, 10.0, 14.0], [0.0, 19.0]);
pub(in crate::entity_models) const CHEST_RIGHT_LID_CUBE: ModelCube =
    chest_cube([1.0, 0.0, 0.0], [15.0, 5.0, 14.0], [0.0, 0.0]);
pub(in crate::entity_models) const CHEST_RIGHT_LOCK_CUBE: ModelCube =
    chest_cube([15.0, -2.0, 14.0], [1.0, 4.0, 1.0], [0.0, 0.0]);

// Vanilla `ChestModel.createDoubleBodyLeftLayer`: the 15-wide left half spans x = 0..15 with the
// seam-facing WEST faces culled in vanilla (same emitted-but-enclosed note as the right half).
pub(in crate::entity_models) const CHEST_LEFT_BOTTOM_CUBE: ModelCube =
    chest_cube([0.0, 0.0, 1.0], [15.0, 10.0, 14.0], [0.0, 19.0]);
pub(in crate::entity_models) const CHEST_LEFT_LID_CUBE: ModelCube =
    chest_cube([0.0, 0.0, 0.0], [15.0, 5.0, 14.0], [0.0, 0.0]);
pub(in crate::entity_models) const CHEST_LEFT_LOCK_CUBE: ModelCube =
    chest_cube([0.0, -2.0, 14.0], [1.0, 4.0, 1.0], [0.0, 0.0]);

/// Vanilla `PartPose.offset(0, 9, 1)` shared by the lid and the lock, so the
/// lock rotates with the lid about the hinge.
pub(in crate::entity_models) const CHEST_LID_POSE: PartPose = PartPose {
    offset: [0.0, 9.0, 1.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Vanilla 26.1 `ChestModel`: the chest block-entity mesh (`bottom`/`lid`/`lock`
/// root children) in its three `ChestType` variants. `setup_anim` transcribes
/// the openness chain `ChestRenderer.submit` + `ChestModel.setupAnim`: the raw
/// combined lid openness `o` is eased as `1 - (1 - o)^3` and the lid and lock
/// rotate `xRot = -(eased * π/2)`.
pub(in crate::entity_models) struct ChestModel {
    root: ModelPart,
}

impl ChestModel {
    pub(in crate::entity_models) fn new(half: ChestModelHalf) -> Self {
        let (bottom, lid, lock) = match half {
            ChestModelHalf::Single => (
                CHEST_SINGLE_BOTTOM_CUBE,
                CHEST_SINGLE_LID_CUBE,
                CHEST_SINGLE_LOCK_CUBE,
            ),
            ChestModelHalf::Left => (
                CHEST_LEFT_BOTTOM_CUBE,
                CHEST_LEFT_LID_CUBE,
                CHEST_LEFT_LOCK_CUBE,
            ),
            ChestModelHalf::Right => (
                CHEST_RIGHT_BOTTOM_CUBE,
                CHEST_RIGHT_LID_CUBE,
                CHEST_RIGHT_LOCK_CUBE,
            ),
        };
        Self {
            root: ModelPart::new(
                PART_POSE_ZERO,
                Vec::new(),
                vec![
                    ("bottom", ModelPart::leaf(PART_POSE_ZERO, vec![bottom])),
                    ("lid", ModelPart::leaf(CHEST_LID_POSE, vec![lid])),
                    ("lock", ModelPart::leaf(CHEST_LID_POSE, vec![lock])),
                ],
            ),
        }
    }
}

/// Vanilla `ChestRenderer.submit` easing over the combined raw openness:
/// `o = 1 - o; o = 1 - o * o * o`.
pub(in crate::entity_models) fn chest_lid_eased_openness(openness: f32) -> f32 {
    let inverted = 1.0 - openness;
    1.0 - inverted * inverted * inverted
}

impl EntityModel for ChestModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        // Vanilla `ChestModel.setupAnim(open)`: `lid.xRot = -(open * π/2)`;
        // `lock.xRot = lid.xRot`, with `open` pre-eased by `ChestRenderer.submit`.
        let x_rot = -(chest_lid_eased_openness(instance.render_state.chest_openness) * FRAC_PI_2);
        self.root.child_mut("lid").pose.rotation[0] = x_rot;
        self.root.child_mut("lock").pose.rotation[0] = x_rot;
    }
}
