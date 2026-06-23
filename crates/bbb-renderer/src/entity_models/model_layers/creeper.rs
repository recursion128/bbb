use super::{apply_head_look, apply_quadruped_leg_swing, PartPose, CREEPER_GREEN, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_CREEPER: &str = "minecraft:creeper#main";

// Vanilla 26.1 `CreeperModel.createBodyLayer(CubeDeformation.NONE)`. Each cube carries both render
// paths' data: the colored debug tint (`CREEPER_GREEN`) and the textured `uv_size` / `texOffs`.
pub(in crate::entity_models) const CREEPER_HEAD: [ModelCube; 1] = [ModelCube::new(
    [-4.0, -8.0, -4.0],
    [8.0, 8.0, 8.0],
    CREEPER_GREEN,
    [8.0, 8.0, 8.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const CREEPER_BODY: [ModelCube; 1] = [ModelCube::new(
    [-4.0, 0.0, -2.0],
    [8.0, 12.0, 4.0],
    CREEPER_GREEN,
    [8.0, 12.0, 4.0],
    [16.0, 16.0],
    false,
)];

pub(in crate::entity_models) const CREEPER_LEG: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, -2.0],
    [4.0, 6.0, 4.0],
    CREEPER_GREEN,
    [4.0, 6.0, 4.0],
    [0.0, 16.0],
    false,
)];

/// The head/body part pose (vanilla `PartPose.offset(0, 6, 0)`).
pub(in crate::entity_models) const CREEPER_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 6.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Builds a creeper leg part at `offset` (vanilla `PartPose.offset`, no rotation).
fn creeper_leg(offset: [f32; 3]) -> ModelPart {
    ModelPart::leaf(
        PartPose {
            offset,
            rotation: [0.0, 0.0, 0.0],
        },
        CREEPER_LEG.to_vec(),
    )
}

/// Mutable creeper model, mirroring vanilla `CreeperModel`. The unified tree is built once with the
/// vanilla `QuadrupedModel` child names: `head`, `body`, then the four legs. `setup_anim` follows the
/// head look ([`apply_head_look`] on `head`) and applies the standard `QuadrupedModel` leg swing
/// ([`apply_quadruped_leg_swing`]). The `CreeperRenderer.scale` swell inflate-and-flicker is
/// folded into the root transform (`creeper_model_root_transform`); the powered charge layer is
/// deferred.
pub(in crate::entity_models) struct CreeperModel {
    root: ModelPart,
}

impl CreeperModel {
    pub(in crate::entity_models) fn new() -> Self {
        let children: Vec<(&'static str, ModelPart)> = vec![
            (
                "head",
                ModelPart::leaf(CREEPER_HEAD_POSE, CREEPER_HEAD.to_vec()),
            ),
            (
                "body",
                ModelPart::leaf(CREEPER_HEAD_POSE, CREEPER_BODY.to_vec()),
            ),
            ("right_hind_leg", creeper_leg([-2.0, 18.0, 4.0])),
            ("left_hind_leg", creeper_leg([2.0, 18.0, 4.0])),
            ("right_front_leg", creeper_leg([-2.0, 18.0, -4.0])),
            ("left_front_leg", creeper_leg([2.0, 18.0, -4.0])),
        ];
        Self {
            root: ModelPart::new(PART_POSE_ZERO, Vec::new(), children),
        }
    }
}

impl EntityModel for CreeperModel {
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
        apply_quadruped_leg_swing(
            &mut self.root,
            render_state.walk_animation_pos,
            render_state.walk_animation_speed,
        );
    }
}
