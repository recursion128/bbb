use super::{PartPose, DRAGON_MEMBRANE, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_ELYTRA: &str = "minecraft:elytra#main";
pub(in crate::entity_models) const MODEL_LAYER_ELYTRA_BABY: &str = "minecraft:elytra_baby#main";
pub(in crate::entity_models) const ELYTRA_DEFAULT_X_ROT: f32 = std::f32::consts::PI / 12.0;
pub(in crate::entity_models) const ELYTRA_DEFAULT_Y_ROT: f32 = 0.0;
pub(in crate::entity_models) const ELYTRA_DEFAULT_Z_ROT: f32 = -std::f32::consts::PI / 12.0;
const ELYTRA_BABY_SCALE: f32 = 0.5;
const ELYTRA_BABY_Y_OFFSET: f32 = 24.016 * (1.0 - ELYTRA_BABY_SCALE);

const ELYTRA_LEFT_WING: [ModelCube; 1] = [elytra_wing_cube([-10.0, 0.0, 0.0], false)];
const ELYTRA_RIGHT_WING: [ModelCube; 1] = [elytra_wing_cube([0.0, 0.0, 0.0], true)];

const ELYTRA_LEFT_WING_POSE: PartPose = PartPose {
    offset: [5.0, 0.0, 0.0],
    rotation: [
        ELYTRA_DEFAULT_X_ROT,
        ELYTRA_DEFAULT_Y_ROT,
        ELYTRA_DEFAULT_Z_ROT,
    ],
};
const ELYTRA_RIGHT_WING_POSE: PartPose = PartPose {
    offset: [-5.0, 0.0, 0.0],
    rotation: [
        ELYTRA_DEFAULT_X_ROT,
        -ELYTRA_DEFAULT_Y_ROT,
        -ELYTRA_DEFAULT_Z_ROT,
    ],
};

/// Vanilla `ElytraModel.createLayer`: both wings use `texOffs(22, 0)` and are grown with
/// `CubeDeformation(1.0F)`, which expands geometry while keeping the base 10x20x2 UV box.
const fn elytra_wing_cube(min: [f32; 3], mirror: bool) -> ModelCube {
    let g = 1.0;
    ModelCube::new(
        [min[0] - g, min[1] - g, min[2] - g],
        [10.0 + 2.0 * g, 20.0 + 2.0 * g, 2.0 + 2.0 * g],
        DRAGON_MEMBRANE,
        [10.0, 20.0, 2.0],
        [22.0, 0.0],
        mirror,
    )
}

fn elytra_tree(baby: bool) -> ModelPart {
    ModelPart::new(
        if baby {
            PartPose {
                offset: [0.0, ELYTRA_BABY_Y_OFFSET, 0.0],
                rotation: [0.0, 0.0, 0.0],
            }
        } else {
            PART_POSE_ZERO
        },
        Vec::new(),
        vec![
            (
                "left_wing",
                ModelPart::leaf(ELYTRA_LEFT_WING_POSE, ELYTRA_LEFT_WING.to_vec()),
            ),
            (
                "right_wing",
                ModelPart::leaf(ELYTRA_RIGHT_WING_POSE, ELYTRA_RIGHT_WING.to_vec()),
            ),
        ],
    )
}

pub(in crate::entity_models) struct ElytraModel {
    root: ModelPart,
    baby: bool,
}

impl ElytraModel {
    pub(in crate::entity_models) fn new(baby: bool) -> Self {
        Self {
            root: elytra_tree(baby),
            baby,
        }
    }
}

impl EntityModel for ElytraModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let state = &instance.render_state;
        if self.baby {
            self.root.scale = [ELYTRA_BABY_SCALE; 3];
        }
        let y = if state.is_crouching { 3.0 } else { 0.0 };
        let left = self.root.child_mut("left_wing");
        left.pose.offset[1] = y;
        left.pose.rotation[0] = state.elytra_rot_x;
        left.pose.rotation[1] = state.elytra_rot_y;
        left.pose.rotation[2] = state.elytra_rot_z;

        let right = self.root.child_mut("right_wing");
        right.pose.offset[1] = y;
        right.pose.rotation[0] = state.elytra_rot_x;
        right.pose.rotation[1] = -state.elytra_rot_y;
        right.pose.rotation[2] = -state.elytra_rot_z;
    }
}
