use super::{
    apply_head_look, enderman_arm_swing_pose, enderman_carried_arm_pose, enderman_leg_swing_pose,
    PartPose, ENDERMAN_DARK, PART_POSE_ZERO,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_ENDERMAN: &str = "minecraft:enderman#main";

// Vanilla 26.1 `EndermanModel.createBodyLayer` cubes (atlas 64×32). Each unified cube carries the
// colored tint (`ENDERMAN_DARK`) and the textured `uv_size`/`texOffs`/`mirror`. The hat's `uv_size`
// keeps the base 8×8×8 box though its geometry is the 7×7×7 inner box (the squid precedent); the
// left arm/leg reuse their right counterpart's `texOffs` mirrored.
pub(in crate::entity_models) const ENDERMAN_HEAD: [ModelCube; 1] = [ModelCube::new(
    [-4.0, -8.0, -4.0],
    [8.0, 8.0, 8.0],
    ENDERMAN_DARK,
    [8.0, 8.0, 8.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const ENDERMAN_HAT: [ModelCube; 1] = [ModelCube::new(
    [-3.5, -7.5, -3.5],
    [7.0, 7.0, 7.0],
    ENDERMAN_DARK,
    [8.0, 8.0, 8.0],
    [0.0, 16.0],
    false,
)];

pub(in crate::entity_models) const ENDERMAN_BODY: [ModelCube; 1] = [ModelCube::new(
    [-4.0, 0.0, -2.0],
    [8.0, 12.0, 4.0],
    ENDERMAN_DARK,
    [8.0, 12.0, 4.0],
    [32.0, 16.0],
    false,
)];

pub(in crate::entity_models) const ENDERMAN_RIGHT_ARM: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -2.0, -1.0],
    [2.0, 30.0, 2.0],
    ENDERMAN_DARK,
    [2.0, 30.0, 2.0],
    [56.0, 0.0],
    false,
)];

pub(in crate::entity_models) const ENDERMAN_LEFT_ARM: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -2.0, -1.0],
    [2.0, 30.0, 2.0],
    ENDERMAN_DARK,
    [2.0, 30.0, 2.0],
    [56.0, 0.0],
    true,
)];

pub(in crate::entity_models) const ENDERMAN_RIGHT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.0],
    [2.0, 30.0, 2.0],
    ENDERMAN_DARK,
    [2.0, 30.0, 2.0],
    [56.0, 0.0],
    false,
)];

pub(in crate::entity_models) const ENDERMAN_LEFT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.0],
    [2.0, 30.0, 2.0],
    ENDERMAN_DARK,
    [2.0, 30.0, 2.0],
    [56.0, 0.0],
    true,
)];

/// The enderman head pose: `PartPose.offset(0, -13, 0)`. The hat child sits at the head's origin.
pub(in crate::entity_models) const ENDERMAN_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, -13.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Builds the enderman tree with the vanilla `EndermanModel` (HumanoidModel) child names: `head`
/// (parenting `hat`), `body`, the two arms, the two legs. The hat is the head's child so the head
/// look and creepy-stare drop re-pose it automatically.
fn enderman_tree() -> ModelPart {
    let head = ModelPart::new(
        ENDERMAN_HEAD_POSE,
        ENDERMAN_HEAD.to_vec(),
        vec![(
            "hat",
            ModelPart::leaf(PART_POSE_ZERO, ENDERMAN_HAT.to_vec()),
        )],
    );
    let children: Vec<(&'static str, ModelPart)> = vec![
        ("head", head),
        (
            "body",
            ModelPart::leaf(
                PartPose {
                    offset: [0.0, -14.0, 0.0],
                    rotation: [0.0, 0.0, 0.0],
                },
                ENDERMAN_BODY.to_vec(),
            ),
        ),
        (
            "right_arm",
            ModelPart::leaf(
                PartPose {
                    offset: [-5.0, -12.0, 0.0],
                    rotation: [0.0, 0.0, 0.0],
                },
                ENDERMAN_RIGHT_ARM.to_vec(),
            ),
        ),
        (
            "left_arm",
            ModelPart::leaf(
                PartPose {
                    offset: [5.0, -12.0, 0.0],
                    rotation: [0.0, 0.0, 0.0],
                },
                ENDERMAN_LEFT_ARM.to_vec(),
            ),
        ),
        (
            "right_leg",
            ModelPart::leaf(
                PartPose {
                    offset: [-2.0, -5.0, 0.0],
                    rotation: [0.0, 0.0, 0.0],
                },
                ENDERMAN_RIGHT_LEG.to_vec(),
            ),
        ),
        (
            "left_leg",
            ModelPart::leaf(
                PartPose {
                    offset: [2.0, -5.0, 0.0],
                    rotation: [0.0, 0.0, 0.0],
                },
                ENDERMAN_LEFT_LEG.to_vec(),
            ),
        ),
    ];
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

/// Mutable enderman model, mirroring vanilla `EndermanModel extends HumanoidModel`. The unified tree
/// is built once with the vanilla HumanoidModel child names. `setup_anim` looks the head, then
/// applies the inherited arm/leg swing plus the arms' always-on idle bob, halved and clamped to
/// `[-0.4, 0.4]` on `xRot` ([`enderman_arm_swing_pose`] on the two arms, [`enderman_leg_swing_pose`]
/// on the two legs); the bob's `zRot` survives the clamp so the arms gently splay.
/// Carrying a block overrides both arms ([`enderman_carried_arm_pose`]); the creepy stare drops the
/// head `y -= 5` and raises its `hat` child `y += 5` (vanilla's `isCreepy` branch), so the outer head
/// layer holds its world position as the inner head opens downward. Both the base and eyes textured
/// passes read this one posed tree.
pub(in crate::entity_models) struct EndermanModel {
    root: ModelPart,
}

impl EndermanModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: enderman_tree(),
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
            self.root.child_mut("head"),
            render_state.head_yaw,
            render_state.head_pitch,
        );
        let limb_swing = render_state.walk_animation_pos;
        let limb_swing_amount = render_state.walk_animation_speed;
        for name in ["right_arm", "left_arm"] {
            let arm = self.root.child_mut(name);
            arm.pose = enderman_arm_swing_pose(
                arm.pose,
                limb_swing,
                limb_swing_amount,
                render_state.age_in_ticks,
            );
        }
        for name in ["right_leg", "left_leg"] {
            let leg = self.root.child_mut(name);
            leg.pose = enderman_leg_swing_pose(leg.pose, limb_swing, limb_swing_amount);
        }
        // Carrying a block overrides the arm swing entirely (held out front).
        if render_state.enderman_carrying {
            for name in ["right_arm", "left_arm"] {
                let arm = self.root.child_mut(name);
                arm.pose = enderman_carried_arm_pose(arm.pose);
            }
        }
        // The creepy stare drops the head and raises its hat child to keep the outer layer in place.
        if render_state.enderman_creepy {
            let head = self.root.child_mut("head");
            head.pose.offset[1] -= 5.0;
            let hat = head.child_mut("hat");
            hat.pose.offset[1] += 5.0;
        }
    }
}
