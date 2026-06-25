use super::{
    apply_crossbow_charge_pose, apply_crossbow_hold_pose, apply_head_look,
    apply_humanoid_attack_animation, apply_humanoid_block_pose, apply_humanoid_bow_pose,
    apply_humanoid_brush_pose, apply_humanoid_crouch_named, apply_humanoid_item_hold_pose,
    apply_humanoid_spyglass_pose, apply_humanoid_stab_attack_animation,
    apply_humanoid_throw_trident_pose, apply_humanoid_toot_horn_pose, apply_humanoid_walk,
    PartPose, CROSSBOW_CHARGE_DURATION_TICKS, PART_POSE_ZERO, PLAYER_BLUE,
};
use crate::entity_models::catalog::PlayerModelPartVisibility;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_PLAYER: &str = "minecraft:player#main";
pub(in crate::entity_models) const MODEL_LAYER_PLAYER_SLIM: &str = "minecraft:player_slim#main";

// Vanilla 26.1 PlayerModel.createMesh(CubeDeformation.NONE, slim). Each cube carries both render
// paths' data: the colored debug tint and the textured uv_size/texOffs/mirror. Each base part nests
// one inflated skin-customization overlay child (hat/jacket/sleeve/pants) that the player part
// visibility toggles; the overlays keep the base box as uv_size.
pub(in crate::entity_models) const PLAYER_HEAD: [ModelCube; 1] = [ModelCube::new(
    [-4.0, -8.0, -4.0],
    [8.0, 8.0, 8.0],
    PLAYER_BLUE,
    [8.0, 8.0, 8.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const PLAYER_HAT: [ModelCube; 1] = [ModelCube::new(
    [-4.5, -8.5, -4.5],
    [9.0, 9.0, 9.0],
    PLAYER_BLUE,
    [8.0, 8.0, 8.0],
    [32.0, 0.0],
    false,
)];

pub(in crate::entity_models) const PLAYER_BODY: [ModelCube; 1] = [ModelCube::new(
    [-4.0, 0.0, -2.0],
    [8.0, 12.0, 4.0],
    PLAYER_BLUE,
    [8.0, 12.0, 4.0],
    [16.0, 16.0],
    false,
)];

pub(in crate::entity_models) const PLAYER_JACKET: [ModelCube; 1] = [ModelCube::new(
    [-4.25, -0.25, -2.25],
    [8.5, 12.5, 4.5],
    PLAYER_BLUE,
    [8.0, 12.0, 4.0],
    [16.0, 32.0],
    false,
)];

pub(in crate::entity_models) const PLAYER_WIDE_RIGHT_ARM: [ModelCube; 1] = [ModelCube::new(
    [-3.0, -2.0, -2.0],
    [4.0, 12.0, 4.0],
    PLAYER_BLUE,
    [4.0, 12.0, 4.0],
    [40.0, 16.0],
    false,
)];

pub(in crate::entity_models) const PLAYER_WIDE_RIGHT_SLEEVE: [ModelCube; 1] = [ModelCube::new(
    [-3.25, -2.25, -2.25],
    [4.5, 12.5, 4.5],
    PLAYER_BLUE,
    [4.0, 12.0, 4.0],
    [40.0, 32.0],
    false,
)];

pub(in crate::entity_models) const PLAYER_WIDE_LEFT_ARM: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -2.0, -2.0],
    [4.0, 12.0, 4.0],
    PLAYER_BLUE,
    [4.0, 12.0, 4.0],
    [32.0, 48.0],
    false,
)];

pub(in crate::entity_models) const PLAYER_WIDE_LEFT_SLEEVE: [ModelCube; 1] = [ModelCube::new(
    [-1.25, -2.25, -2.25],
    [4.5, 12.5, 4.5],
    PLAYER_BLUE,
    [4.0, 12.0, 4.0],
    [48.0, 48.0],
    false,
)];

pub(in crate::entity_models) const PLAYER_SLIM_RIGHT_ARM: [ModelCube; 1] = [ModelCube::new(
    [-2.0, -2.0, -2.0],
    [3.0, 12.0, 4.0],
    PLAYER_BLUE,
    [3.0, 12.0, 4.0],
    [40.0, 16.0],
    false,
)];

pub(in crate::entity_models) const PLAYER_SLIM_RIGHT_SLEEVE: [ModelCube; 1] = [ModelCube::new(
    [-2.25, -2.25, -2.25],
    [3.5, 12.5, 4.5],
    PLAYER_BLUE,
    [3.0, 12.0, 4.0],
    [40.0, 32.0],
    false,
)];

pub(in crate::entity_models) const PLAYER_SLIM_LEFT_ARM: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -2.0, -2.0],
    [3.0, 12.0, 4.0],
    PLAYER_BLUE,
    [3.0, 12.0, 4.0],
    [32.0, 48.0],
    false,
)];

pub(in crate::entity_models) const PLAYER_SLIM_LEFT_SLEEVE: [ModelCube; 1] = [ModelCube::new(
    [-1.25, -2.25, -2.25],
    [3.5, 12.5, 4.5],
    PLAYER_BLUE,
    [3.0, 12.0, 4.0],
    [48.0, 48.0],
    false,
)];

pub(in crate::entity_models) const PLAYER_RIGHT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, -2.0],
    [4.0, 12.0, 4.0],
    PLAYER_BLUE,
    [4.0, 12.0, 4.0],
    [0.0, 16.0],
    false,
)];

pub(in crate::entity_models) const PLAYER_LEFT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, -2.0],
    [4.0, 12.0, 4.0],
    PLAYER_BLUE,
    [4.0, 12.0, 4.0],
    [16.0, 48.0],
    false,
)];

pub(in crate::entity_models) const PLAYER_RIGHT_PANTS: [ModelCube; 1] = [ModelCube::new(
    [-2.25, -0.25, -2.25],
    [4.5, 12.5, 4.5],
    PLAYER_BLUE,
    [4.0, 12.0, 4.0],
    [0.0, 32.0],
    false,
)];

pub(in crate::entity_models) const PLAYER_LEFT_PANTS: [ModelCube; 1] = [ModelCube::new(
    [-2.25, -0.25, -2.25],
    [4.5, 12.5, 4.5],
    PLAYER_BLUE,
    [4.0, 12.0, 4.0],
    [0.0, 48.0],
    false,
)];

/// Shared humanoid limb part poses (vanilla `PlayerModel.createMesh`).
const PLAYER_RIGHT_ARM_POSE: PartPose = PartPose {
    offset: [-5.0, 2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const PLAYER_LEFT_ARM_POSE: PartPose = PartPose {
    offset: [5.0, 2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const PLAYER_RIGHT_LEG_POSE: PartPose = PartPose {
    offset: [-1.9, 12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const PLAYER_LEFT_LEG_POSE: PartPose = PartPose {
    offset: [1.9, 12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Builds a base part at `pose` carrying `cubes` plus its single inflated skin overlay child named
/// `overlay_name` (hat/jacket/sleeve/pants) at the zero pose.
fn part_with_overlay(
    pose: PartPose,
    cubes: &[ModelCube],
    overlay_name: &'static str,
    overlay: &[ModelCube],
) -> ModelPart {
    ModelPart::new(
        pose,
        cubes.to_vec(),
        vec![(
            overlay_name,
            ModelPart::leaf(PART_POSE_ZERO, overlay.to_vec()),
        )],
    )
}

/// Builds the unified player root for the `slim`/wide arm model, with the vanilla `HumanoidModel`
/// child names (`head`, `body`, `right_arm`, `left_arm`, `right_leg`, `left_leg`, head first). Each base
/// part nests its one skin overlay child (`hat`/`jacket`/`sleeve`/`pants`) that
/// [`PlayerModel::apply_part_visibility`] toggles.
fn player_tree(slim: bool) -> ModelPart {
    let (right_arm, right_sleeve, left_arm, left_sleeve) = if slim {
        (
            PLAYER_SLIM_RIGHT_ARM.as_slice(),
            PLAYER_SLIM_RIGHT_SLEEVE.as_slice(),
            PLAYER_SLIM_LEFT_ARM.as_slice(),
            PLAYER_SLIM_LEFT_SLEEVE.as_slice(),
        )
    } else {
        (
            PLAYER_WIDE_RIGHT_ARM.as_slice(),
            PLAYER_WIDE_RIGHT_SLEEVE.as_slice(),
            PLAYER_WIDE_LEFT_ARM.as_slice(),
            PLAYER_WIDE_LEFT_SLEEVE.as_slice(),
        )
    };
    let children = vec![
        (
            "head",
            part_with_overlay(PART_POSE_ZERO, &PLAYER_HEAD, "hat", &PLAYER_HAT),
        ),
        (
            "body",
            part_with_overlay(PART_POSE_ZERO, &PLAYER_BODY, "jacket", &PLAYER_JACKET),
        ),
        (
            "right_arm",
            part_with_overlay(PLAYER_RIGHT_ARM_POSE, right_arm, "sleeve", right_sleeve),
        ),
        (
            "left_arm",
            part_with_overlay(PLAYER_LEFT_ARM_POSE, left_arm, "sleeve", left_sleeve),
        ),
        (
            "right_leg",
            part_with_overlay(
                PLAYER_RIGHT_LEG_POSE,
                &PLAYER_RIGHT_LEG,
                "pants",
                &PLAYER_RIGHT_PANTS,
            ),
        ),
        (
            "left_leg",
            part_with_overlay(
                PLAYER_LEFT_LEG_POSE,
                &PLAYER_LEFT_LEG,
                "pants",
                &PLAYER_LEFT_PANTS,
            ),
        ),
    ];
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

/// Mutable player model, mirroring vanilla `PlayerModel extends HumanoidModel`. The unified tree is
/// built for the `slim`/wide arm model with the vanilla child names; each of the six base parts (head,
/// body, arms, legs) carries one skin overlay child (hat/jacket/sleeve/pants). `setup_anim` looks the
/// head, runs the inherited `HumanoidModel` walk swing + idle arm bob ([`apply_humanoid_walk`]),
/// the crouch sneaking pose ([`apply_humanoid_crouch_named`]), then the melee swing
/// ([`apply_humanoid_attack_animation`], vanilla `setupAttackAnimation`). The held-item arm pose, the
/// per-item swing types (STAB / NONE), swim, and the elytra defer.
pub(in crate::entity_models) struct PlayerModel {
    root: ModelPart,
}

impl PlayerModel {
    pub(in crate::entity_models) fn new(slim: bool) -> Self {
        Self {
            root: player_tree(slim),
        }
    }

    /// Toggles the six skin-customization overlay children (hat/jacket/right & left sleeve/right &
    /// left pants), which the base parts each carry as their single named child, by the player's
    /// `PlayerModelPartVisibility`. The textured path calls this after [`EntityModel::prepare`]; the
    /// colored fallback leaves every overlay visible (vanilla renders untextured players whole).
    pub(in crate::entity_models) fn apply_part_visibility(
        &mut self,
        parts: PlayerModelPartVisibility,
    ) {
        let overlays = [
            ("head", "hat", parts.hat),
            ("body", "jacket", parts.jacket),
            ("right_arm", "sleeve", parts.right_sleeve),
            ("left_arm", "sleeve", parts.left_sleeve),
            ("right_leg", "pants", parts.right_pants),
            ("left_leg", "pants", parts.left_pants),
        ];
        for (base, overlay, visible) in overlays {
            self.root.child_mut(base).child_mut(overlay).visible = visible;
        }
    }
}

impl EntityModel for PlayerModel {
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
        apply_humanoid_walk(
            &mut self.root,
            render_state.walk_animation_pos,
            render_state.walk_animation_speed,
            render_state.age_in_ticks,
        );
        // Vanilla `HumanoidModel.setupAnim` poses the using arm (here SPYGLASS) BEFORE the crouch block,
        // so the crouch `arm.xRot += 0.4` still lands on top of the raised arm.
        if render_state.player_using_spyglass {
            apply_humanoid_spyglass_pose(
                &mut self.root,
                render_state.head_yaw,
                render_state.head_pitch,
                render_state.use_item_off_hand,
                render_state.is_crouching,
            );
        }
        if render_state.player_tooting_horn {
            apply_humanoid_toot_horn_pose(
                &mut self.root,
                render_state.head_yaw,
                render_state.head_pitch,
                render_state.use_item_off_hand,
            );
        }
        if render_state.player_brushing {
            apply_humanoid_brush_pose(&mut self.root, render_state.use_item_off_hand);
        }
        if render_state.player_blocking {
            apply_humanoid_block_pose(
                &mut self.root,
                render_state.head_yaw,
                render_state.head_pitch,
                render_state.use_item_off_hand,
            );
        }
        if render_state.player_throwing_trident {
            apply_humanoid_throw_trident_pose(&mut self.root, render_state.use_item_off_hand);
        }
        // Vanilla `HumanoidModel.poseRightArm` `BOW_AND_ARROW`: drawing a main-hand bow raises both arms along
        // the head look. Two-handed + affectsOffhandPose, so `poseLeftArm` is skipped (the projection
        // suppresses the off-hand ITEM fallback while drawing), and this sets both arms here.
        if render_state.player_drawing_bow {
            apply_humanoid_bow_pose(
                &mut self.root,
                render_state.head_yaw,
                render_state.head_pitch,
            );
        }
        // Vanilla `HumanoidModel.poseRightArm` `CROSSBOW_CHARGE` (`AnimationUtils.animateCrossbowCharge`):
        // drawing a main-hand crossbow braces the right arm and pulls the string back with the left over the
        // draw ticks. Two-handed + affectsOffhandPose (off-hand ITEM suppressed in the projection); reuses the
        // shared pillager/piglin pose helper.
        if render_state.player_charging_crossbow {
            apply_crossbow_charge_pose(
                &mut self.root,
                CROSSBOW_CHARGE_DURATION_TICKS,
                render_state.crossbow_charge_ticks,
            );
        }
        // Vanilla `AvatarRenderer.getArmPose` fallback `ITEM`: a player holding a plain main-hand item lowers
        // and halves the arm. It runs as part of `poseRightArm` (before crouch/attack) and only on the main
        // (right) arm, so this is unconditionally the right arm.
        if render_state.player_main_hand_item_pose {
            apply_humanoid_item_hold_pose(&mut self.root, false);
        }
        // Vanilla `AvatarRenderer.getArmPose(_, OFF_HAND)` fallback `ITEM`, posed by `poseLeftArm` onto the
        // OFF (left) arm. Independent of the main-hand pose (separate arm), so order between them is moot.
        if render_state.player_off_hand_item_pose {
            apply_humanoid_item_hold_pose(&mut self.root, true);
        }
        // Vanilla `AvatarRenderer.getArmPose` `CROSSBOW_HOLD` (`animateCrossbowHold`): a charged main-hand
        // crossbow levels along the head look, setting BOTH arms. It is two-handed + affectsOffhandPose and
        // vanilla runs `poseRightArm` last for this case, overwriting any off-hand ITEM, so it is applied
        // after the ITEM blocks (and before crouch/attack).
        if render_state.player_crossbow_hold {
            apply_crossbow_hold_pose(
                &mut self.root,
                render_state.head_yaw,
                render_state.head_pitch,
            );
        }
        if render_state.is_crouching {
            apply_humanoid_crouch_named(&mut self.root);
        }
        // Vanilla `HumanoidModel.setupAnim` runs `setupAttackAnimation` last (after the pose / crouch):
        // a swinging player twists the body and drives the attacking arm. The player is always adult
        // (`ageScale = 1.0`). A held spear lunges (`STAB`); everything else chops (`WHACK`).
        if render_state.main_hand_swing_is_stab {
            apply_humanoid_stab_attack_animation(
                &mut self.root,
                render_state.attack_anim,
                render_state.attack_arm_off_hand,
                1.0,
            );
        } else {
            apply_humanoid_attack_animation(
                &mut self.root,
                render_state.attack_anim,
                render_state.attack_arm_off_hand,
                render_state.head_pitch,
                1.0,
            );
        }
    }
}
