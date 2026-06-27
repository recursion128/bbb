use super::{PartPose, PART_POSE_ZERO, PLAYER_BLUE};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// Vanilla 26.1 `SpinAttackEffectModel.createLayer` (`ModelLayers.PLAYER_SPIN_ATTACK`, atlas 64x64):
// two cuboid shells named `box0`/`box1`, both at `PartPose.ZERO.withScale(0.75 * (i + 1))`.
// `SpinAttackEffectLayer` only submits this model for `AvatarRenderState.isAutoSpinAttack`, using
// `textures/entity/trident/trident_riptide.png` and the default `EntityModel` render type
// (`entityCutout`).

pub(in crate::entity_models) const MODEL_LAYER_PLAYER_SPIN_ATTACK: &str =
    "minecraft:spin_attack#main";

pub(in crate::entity_models) const SPIN_ATTACK_BOX0_CUBE: ModelCube = ModelCube::new(
    [-8.0, -9.6, -8.0],
    [16.0, 32.0, 16.0],
    PLAYER_BLUE,
    [16.0, 32.0, 16.0],
    [0.0, 0.0],
    false,
);
pub(in crate::entity_models) const SPIN_ATTACK_BOX1_CUBE: ModelCube = ModelCube::new(
    [-8.0, 0.0, -8.0],
    [16.0, 32.0, 16.0],
    PLAYER_BLUE,
    [16.0, 32.0, 16.0],
    [0.0, 0.0],
    false,
);

pub(in crate::entity_models) fn spin_attack_box_scale(index: usize) -> f32 {
    0.75 * (index as f32 + 1.0)
}

pub(in crate::entity_models) fn spin_attack_box_yrot(age_in_ticks: f32, index: usize) -> f32 {
    (age_in_ticks * -(45.0 + (index as f32 + 1.0) * 5.0)).to_radians()
}

pub(in crate::entity_models) struct SpinAttackEffectModel {
    root: ModelPart,
}

impl SpinAttackEffectModel {
    pub(in crate::entity_models) fn new() -> Self {
        let mut box0 = ModelPart::leaf(PART_POSE_ZERO, vec![SPIN_ATTACK_BOX0_CUBE]);
        box0.scale = [spin_attack_box_scale(0); 3];
        let mut box1 = ModelPart::leaf(PART_POSE_ZERO, vec![SPIN_ATTACK_BOX1_CUBE]);
        box1.scale = [spin_attack_box_scale(1); 3];
        Self {
            root: ModelPart::new(
                PART_POSE_ZERO,
                Vec::new(),
                vec![("box0", box0), ("box1", box1)],
            ),
        }
    }
}

impl EntityModel for SpinAttackEffectModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        for index in 0..2 {
            let child = self
                .root
                .child_mut(if index == 0 { "box0" } else { "box1" });
            child.scale = [spin_attack_box_scale(index); 3];
            child.pose = PartPose {
                rotation: [
                    0.0,
                    spin_attack_box_yrot(instance.render_state.age_in_ticks, index),
                    0.0,
                ],
                ..PART_POSE_ZERO
            };
        }
    }
}
