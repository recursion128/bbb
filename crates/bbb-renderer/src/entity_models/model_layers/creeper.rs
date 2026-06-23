use super::{
    apply_head_look, apply_quadruped_leg_swing, ModelCubeDesc, ModelPartDesc, PartPose,
    TexturedModelCubeDesc, TexturedModelPartDesc, CREEPER_GREEN,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_CREEPER: &str = "minecraft:creeper#main";

pub(in crate::entity_models) const CREEPER_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -8.0, -4.0],
    size: [8.0, 8.0, 8.0],
    color: CREEPER_GREEN,
}];

pub(in crate::entity_models) const CREEPER_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -2.0],
    size: [8.0, 12.0, 4.0],
    color: CREEPER_GREEN,
}];

pub(in crate::entity_models) const CREEPER_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 6.0, 4.0],
    color: CREEPER_GREEN,
}];

// Vanilla 26.1 CreeperModel.createBodyLayer(CubeDeformation.NONE).
pub(in crate::entity_models) const CREEPER_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 6.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &CREEPER_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 6.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &CREEPER_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 18.0, 4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &CREEPER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 18.0, 4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &CREEPER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 18.0, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &CREEPER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 18.0, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &CREEPER_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const CREEPER_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, -8.0, -4.0],
        size: [8.0, 8.0, 8.0],
        uv_size: [8.0, 8.0, 8.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const CREEPER_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, 0.0, -2.0],
        size: [8.0, 12.0, 4.0],
        uv_size: [8.0, 12.0, 4.0],
        tex: [16.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const CREEPER_TEXTURED_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, 0.0, -2.0],
        size: [4.0, 6.0, 4.0],
        uv_size: [4.0, 6.0, 4.0],
        tex: [0.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const CREEPER_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: CREEPER_PARTS[0].pose,
        cubes: &CREEPER_TEXTURED_HEAD,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: CREEPER_PARTS[1].pose,
        cubes: &CREEPER_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: CREEPER_PARTS[2].pose,
        cubes: &CREEPER_TEXTURED_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: CREEPER_PARTS[3].pose,
        cubes: &CREEPER_TEXTURED_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: CREEPER_PARTS[4].pose,
        cubes: &CREEPER_TEXTURED_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: CREEPER_PARTS[5].pose,
        cubes: &CREEPER_TEXTURED_LEG,
        children: &[],
    },
];

/// Vanilla `CreeperModel` leg part indices: the mesh lists head, body, then the four legs.
const CREEPER_LEG_PART_INDICES: [usize; 4] = [2, 3, 4, 5];

/// Mutable creeper model, mirroring vanilla `CreeperModel`. The unified tree is zipped from the baked
/// colored ([`CREEPER_PARTS`]) and textured ([`CREEPER_TEXTURED_PARTS`]) trees: child 0 is the head,
/// child 1 the body, children 2..=5 the legs. `setup_anim` follows the head look ([`apply_head_look`])
/// and applies the standard `QuadrupedModel` leg swing ([`apply_quadruped_leg_swing`]). The
/// `CreeperRenderer.scale` swell inflate-and-flicker is folded into the root transform
/// (`creeper_model_root_transform`); the powered charge layer is deferred.
pub(in crate::entity_models) struct CreeperModel {
    root: ModelPart,
}

impl CreeperModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::root_from_descs(&CREEPER_PARTS, &CREEPER_TEXTURED_PARTS),
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
            self.root.child_at_mut(0),
            render_state.head_yaw,
            render_state.head_pitch,
        );
        apply_quadruped_leg_swing(
            &mut self.root,
            CREEPER_LEG_PART_INDICES,
            render_state.walk_animation_pos,
            render_state.walk_animation_speed,
        );
    }
}
