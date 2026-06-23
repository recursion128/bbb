use super::{
    apply_head_look, enderman_arm_swing_pose, enderman_carried_arm_pose, enderman_leg_swing_pose,
    head_first_part_index, ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc,
    TexturedModelPartDesc, ENDERMAN_DARK, PART_POSE_ZERO,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_ENDERMAN: &str = "minecraft:enderman#main";

pub(in crate::entity_models) const ENDERMAN_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -8.0, -4.0],
    size: [8.0, 8.0, 8.0],
    color: ENDERMAN_DARK,
}];

pub(in crate::entity_models) const ENDERMAN_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.5, -7.5, -3.5],
    size: [7.0, 7.0, 7.0],
    color: ENDERMAN_DARK,
}];

pub(in crate::entity_models) const ENDERMAN_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -2.0],
    size: [8.0, 12.0, 4.0],
    color: ENDERMAN_DARK,
}];

pub(in crate::entity_models) const ENDERMAN_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, -1.0],
    size: [2.0, 30.0, 2.0],
    color: ENDERMAN_DARK,
}];

pub(in crate::entity_models) const ENDERMAN_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.0],
    size: [2.0, 30.0, 2.0],
    color: ENDERMAN_DARK,
}];

pub(in crate::entity_models) const ENDERMAN_HEAD_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &ENDERMAN_HAT,
    children: &[],
}];

// Vanilla 26.1 EndermanModel.createBodyLayer().
pub(in crate::entity_models) const ENDERMAN_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -13.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ENDERMAN_HEAD,
        children: &ENDERMAN_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -14.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ENDERMAN_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, -12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ENDERMAN_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, -12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ENDERMAN_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, -5.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ENDERMAN_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, -5.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ENDERMAN_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const ENDERMAN_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, -8.0, -4.0],
        size: [8.0, 8.0, 8.0],
        uv_size: [8.0, 8.0, 8.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ENDERMAN_TEXTURED_HAT: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.5, -7.5, -3.5],
        size: [7.0, 7.0, 7.0],
        uv_size: [8.0, 8.0, 8.0],
        tex: [0.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ENDERMAN_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ENDERMAN_TEXTURED_HAT,
        children: &[],
    }];

pub(in crate::entity_models) const ENDERMAN_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, 0.0, -2.0],
        size: [8.0, 12.0, 4.0],
        uv_size: [8.0, 12.0, 4.0],
        tex: [32.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ENDERMAN_TEXTURED_RIGHT_ARM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -2.0, -1.0],
        size: [2.0, 30.0, 2.0],
        uv_size: [2.0, 30.0, 2.0],
        tex: [56.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ENDERMAN_TEXTURED_LEFT_ARM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -2.0, -1.0],
        size: [2.0, 30.0, 2.0],
        uv_size: [2.0, 30.0, 2.0],
        tex: [56.0, 0.0],
        mirror: true,
    }];

pub(in crate::entity_models) const ENDERMAN_TEXTURED_RIGHT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, 30.0, 2.0],
        uv_size: [2.0, 30.0, 2.0],
        tex: [56.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ENDERMAN_TEXTURED_LEFT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, 30.0, 2.0],
        uv_size: [2.0, 30.0, 2.0],
        tex: [56.0, 0.0],
        mirror: true,
    }];

pub(in crate::entity_models) const ENDERMAN_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: ENDERMAN_PARTS[0].pose,
        cubes: &ENDERMAN_TEXTURED_HEAD,
        children: &ENDERMAN_TEXTURED_HEAD_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: ENDERMAN_PARTS[1].pose,
        cubes: &ENDERMAN_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ENDERMAN_PARTS[2].pose,
        cubes: &ENDERMAN_TEXTURED_RIGHT_ARM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ENDERMAN_PARTS[3].pose,
        cubes: &ENDERMAN_TEXTURED_LEFT_ARM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ENDERMAN_PARTS[4].pose,
        cubes: &ENDERMAN_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ENDERMAN_PARTS[5].pose,
        cubes: &ENDERMAN_TEXTURED_LEFT_LEG,
        children: &[],
    },
];

/// Mutable enderman model, mirroring vanilla `EndermanModel extends HumanoidModel`. The unified tree
/// is zipped from the colored ([`ENDERMAN_PARTS`]) and textured ([`ENDERMAN_TEXTURED_PARTS`]) const
/// trees. `setup_anim` looks the head (part `0`), then applies the inherited arm/leg swing halved and
/// clamped to `[-0.4, 0.4]` ([`enderman_arm_swing_pose`] at `[2, 3]`, [`enderman_leg_swing_pose`] at
/// `[4, 5]`). Carrying a block overrides both arms ([`enderman_carried_arm_pose`]); the creepy stare
/// drops the head `y -= 5` and raises its hat child `y += 5` (vanilla's `isCreepy` branch), so the
/// outer head layer holds its world position as the inner head opens downward. Both the base and
/// eyes textured passes read this one posed tree.
pub(in crate::entity_models) struct EndermanModel {
    root: ModelPart,
}

impl EndermanModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::root_from_descs(&ENDERMAN_PARTS, &ENDERMAN_TEXTURED_PARTS),
        }
    }
}

impl EntityModel for EndermanModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let render_state = &instance.render_state;
        apply_head_look(
            self.root.child_at_mut(head_first_part_index()),
            render_state.head_yaw,
            render_state.head_pitch,
        );
        let limb_swing = render_state.walk_animation_pos;
        let limb_swing_amount = render_state.walk_animation_speed;
        for index in [2, 3] {
            let arm = self.root.child_at_mut(index);
            arm.pose = enderman_arm_swing_pose(arm.pose, limb_swing, limb_swing_amount);
        }
        for index in [4, 5] {
            let leg = self.root.child_at_mut(index);
            leg.pose = enderman_leg_swing_pose(leg.pose, limb_swing, limb_swing_amount);
        }
        // Carrying a block overrides the arm swing entirely (held out front).
        if render_state.enderman_carrying {
            for index in [2, 3] {
                let arm = self.root.child_at_mut(index);
                arm.pose = enderman_carried_arm_pose(arm.pose);
            }
        }
        // The creepy stare drops the head and raises its hat child to keep the outer layer in place.
        if render_state.enderman_creepy {
            let head = self.root.child_at_mut(head_first_part_index());
            head.pose.offset[1] -= 5.0;
            let hat = head.child_at_mut(0);
            hat.pose.offset[1] += 5.0;
        }
    }
}
