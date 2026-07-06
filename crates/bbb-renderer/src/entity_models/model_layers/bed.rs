use std::f32::consts::{FRAC_PI_2, PI};

use super::{
    PartPose, BED_WOOL, MODEL_CUBE_FACES_ALL, MODEL_CUBE_FACE_DOWN, MODEL_CUBE_FACE_UP,
    PART_POSE_ZERO,
};
use crate::entity_models::catalog::BedModelPart;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// Vanilla `ModelLayers.BED_HEAD` / `BED_FOOT` (`register("bed_head")` / `register("bed_foot")`).
pub(in crate::entity_models) const MODEL_LAYER_BED_HEAD: &str = "minecraft:bed_head#main";
pub(in crate::entity_models) const MODEL_LAYER_BED_FOOT: &str = "minecraft:bed_foot#main";

const fn bed_cube(min: [f32; 3], size: [f32; 3], tex: [f32; 2], visible_faces: u8) -> ModelCube {
    ModelCube::new(min, size, BED_WOOL, size, tex, false).with_visible_faces(visible_faces)
}

// Vanilla 26.1 `BedRenderer.createHeadLayer` (atlas 64×64): the 16×16×6 `main` slab at
// texOffs(0, 0) with `Util.allOfEnumExcept(Direction.UP)` — the hidden UP face is the head/foot
// seam plane — plus two 3×3×3 legs at texOffs(50, 6) / (50, 18) with
// `VISIBLE_LEG_FACES = allOfEnumExcept(Direction.DOWN)` (the hidden DOWN face presses flat
// against the mattress underside and would z-fight it). Like the chest, the bed mesh is
// authored without the entity `scale(-1, -1, 1)` flip; `BedRenderer.createModelTransform` lays
// it flat with `Rx(90°)`.
pub(in crate::entity_models) const BED_HEAD_MAIN_CUBE: ModelCube = bed_cube(
    [0.0, 0.0, 0.0],
    [16.0, 16.0, 6.0],
    [0.0, 0.0],
    MODEL_CUBE_FACES_ALL & !MODEL_CUBE_FACE_UP,
);
pub(in crate::entity_models) const BED_HEAD_LEFT_LEG_CUBE: ModelCube = bed_cube(
    [0.0, 6.0, 0.0],
    [3.0, 3.0, 3.0],
    [50.0, 6.0],
    MODEL_CUBE_FACES_ALL & !MODEL_CUBE_FACE_DOWN,
);
pub(in crate::entity_models) const BED_HEAD_RIGHT_LEG_CUBE: ModelCube = bed_cube(
    [-16.0, 6.0, 0.0],
    [3.0, 3.0, 3.0],
    [50.0, 18.0],
    MODEL_CUBE_FACES_ALL & !MODEL_CUBE_FACE_DOWN,
);

// Vanilla `BedRenderer.createFootLayer`: the 16×16×6 `main` slab at texOffs(0, 22) with
// `allOfEnumExcept(Direction.DOWN)` (the hidden DOWN face is the foot side of the same seam
// plane) plus two legs at texOffs(50, 0) / (50, 12).
pub(in crate::entity_models) const BED_FOOT_MAIN_CUBE: ModelCube = bed_cube(
    [0.0, 0.0, 0.0],
    [16.0, 16.0, 6.0],
    [0.0, 22.0],
    MODEL_CUBE_FACES_ALL & !MODEL_CUBE_FACE_DOWN,
);
pub(in crate::entity_models) const BED_FOOT_LEFT_LEG_CUBE: ModelCube = bed_cube(
    [0.0, 6.0, -16.0],
    [3.0, 3.0, 3.0],
    [50.0, 0.0],
    MODEL_CUBE_FACES_ALL & !MODEL_CUBE_FACE_DOWN,
);
pub(in crate::entity_models) const BED_FOOT_RIGHT_LEG_CUBE: ModelCube = bed_cube(
    [-16.0, 6.0, -16.0],
    [3.0, 3.0, 3.0],
    [50.0, 12.0],
    MODEL_CUBE_FACES_ALL & !MODEL_CUBE_FACE_DOWN,
);

const fn bed_leg_pose(z_rot: f32) -> PartPose {
    // Vanilla `PartPose.rotation(π/2, 0, zRot)`: every leg shares the `π/2` X rotation, the
    // Z rotation walks the four corners.
    PartPose {
        offset: [0.0, 0.0, 0.0],
        rotation: [FRAC_PI_2, 0.0, z_rot],
    }
}

/// Vanilla `PartPose.rotation((float) (Math.PI / 2), 0, (float) (Math.PI / 2))` (head left leg).
pub(in crate::entity_models) const BED_HEAD_LEFT_LEG_POSE: PartPose = bed_leg_pose(FRAC_PI_2);
/// Vanilla `PartPose.rotation((float) (Math.PI / 2), 0, (float) Math.PI)` (head right leg).
pub(in crate::entity_models) const BED_HEAD_RIGHT_LEG_POSE: PartPose = bed_leg_pose(PI);
/// Vanilla `PartPose.rotation((float) (Math.PI / 2), 0, 0)` (foot left leg).
pub(in crate::entity_models) const BED_FOOT_LEFT_LEG_POSE: PartPose = bed_leg_pose(0.0);
/// Vanilla `PartPose.rotation((float) (Math.PI / 2), 0, (float) (Math.PI * 3.0 / 2.0))`
/// (foot right leg).
pub(in crate::entity_models) const BED_FOOT_RIGHT_LEG_POSE: PartPose = bed_leg_pose(PI * 3.0 / 2.0);

/// Vanilla 26.1 `BedRenderer.createHeadLayer` / `createFootLayer`: the bed block-entity mesh
/// (`main` / `left_leg` / `right_leg` root children) per `BedPart`. The bed has no animation
/// (`Model.Simple` with `Unit.INSTANCE` state); facing rides the root transform.
pub(in crate::entity_models) struct BedModel {
    root: ModelPart,
}

impl BedModel {
    pub(in crate::entity_models) fn new(part: BedModelPart) -> Self {
        let (main, left_leg, right_leg, left_pose, right_pose) = match part {
            BedModelPart::Head => (
                BED_HEAD_MAIN_CUBE,
                BED_HEAD_LEFT_LEG_CUBE,
                BED_HEAD_RIGHT_LEG_CUBE,
                BED_HEAD_LEFT_LEG_POSE,
                BED_HEAD_RIGHT_LEG_POSE,
            ),
            BedModelPart::Foot => (
                BED_FOOT_MAIN_CUBE,
                BED_FOOT_LEFT_LEG_CUBE,
                BED_FOOT_RIGHT_LEG_CUBE,
                BED_FOOT_LEFT_LEG_POSE,
                BED_FOOT_RIGHT_LEG_POSE,
            ),
        };
        Self {
            root: ModelPart::new(
                PART_POSE_ZERO,
                Vec::new(),
                vec![
                    ("main", ModelPart::leaf(PART_POSE_ZERO, vec![main])),
                    ("left_leg", ModelPart::leaf(left_pose, vec![left_leg])),
                    ("right_leg", ModelPart::leaf(right_pose, vec![right_leg])),
                ],
            ),
        }
    }
}

impl EntityModel for BedModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, _instance: &EntityModelInstance) {}
}
