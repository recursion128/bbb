use super::{
    apply_head_look, apply_iron_golem_walk, ModelCubeDesc, ModelPartDesc, PartPose,
    TexturedModelCubeDesc, TexturedModelPartDesc, IRON_GOLEM_STONE, SNOW_GOLEM_WHITE,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

pub(in crate::entity_models) const IRON_GOLEM_HEAD: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-4.0, -12.0, -5.5],
        size: [8.0, 10.0, 8.0],
        color: IRON_GOLEM_STONE,
    },
    ModelCubeDesc {
        min: [-1.0, -5.0, -7.5],
        size: [2.0, 4.0, 2.0],
        color: IRON_GOLEM_STONE,
    },
];

pub(in crate::entity_models) const IRON_GOLEM_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-9.0, -2.0, -6.0],
        size: [18.0, 12.0, 11.0],
        color: IRON_GOLEM_STONE,
    },
    ModelCubeDesc {
        min: [-5.0, 9.5, -3.5],
        size: [10.0, 6.0, 7.0],
        color: IRON_GOLEM_STONE,
    },
];

pub(in crate::entity_models) const IRON_GOLEM_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-13.0, -2.5, -3.0],
    size: [4.0, 30.0, 6.0],
    color: IRON_GOLEM_STONE,
}];

pub(in crate::entity_models) const IRON_GOLEM_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [9.0, -2.5, -3.0],
    size: [4.0, 30.0, 6.0],
    color: IRON_GOLEM_STONE,
}];

pub(in crate::entity_models) const IRON_GOLEM_RIGHT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.5, -3.0, -3.0],
    size: [6.0, 16.0, 5.0],
    color: IRON_GOLEM_STONE,
}];

pub(in crate::entity_models) const IRON_GOLEM_LEFT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.5, -3.0, -3.0],
    size: [6.0, 16.0, 5.0],
    color: IRON_GOLEM_STONE,
}];

// Vanilla 26.1 IronGolemModel.createBodyLayer().
pub(in crate::entity_models) const IRON_GOLEM_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -7.0, -2.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &IRON_GOLEM_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -7.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &IRON_GOLEM_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -7.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &IRON_GOLEM_RIGHT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -7.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &IRON_GOLEM_LEFT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 11.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &IRON_GOLEM_RIGHT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, 11.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &IRON_GOLEM_LEFT_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const MODEL_LAYER_IRON_GOLEM: &str = "minecraft:iron_golem#main";

pub(in crate::entity_models) const IRON_GOLEM_TEXTURED_HEAD: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-4.0, -12.0, -5.5],
        size: [8.0, 10.0, 8.0],
        uv_size: [8.0, 10.0, 8.0],
        tex: [0.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-1.0, -5.0, -7.5],
        size: [2.0, 4.0, 2.0],
        uv_size: [2.0, 4.0, 2.0],
        tex: [24.0, 0.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const IRON_GOLEM_TEXTURED_BODY: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-9.0, -2.0, -6.0],
        size: [18.0, 12.0, 11.0],
        uv_size: [18.0, 12.0, 11.0],
        tex: [0.0, 40.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-5.0, 9.5, -3.5],
        size: [10.0, 6.0, 7.0],
        uv_size: [9.0, 5.0, 6.0],
        tex: [0.0, 70.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const IRON_GOLEM_TEXTURED_RIGHT_ARM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-13.0, -2.5, -3.0],
        size: [4.0, 30.0, 6.0],
        uv_size: [4.0, 30.0, 6.0],
        tex: [60.0, 21.0],
        mirror: false,
    }];

pub(in crate::entity_models) const IRON_GOLEM_TEXTURED_LEFT_ARM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [9.0, -2.5, -3.0],
        size: [4.0, 30.0, 6.0],
        uv_size: [4.0, 30.0, 6.0],
        tex: [60.0, 58.0],
        mirror: false,
    }];

pub(in crate::entity_models) const IRON_GOLEM_TEXTURED_RIGHT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.5, -3.0, -3.0],
        size: [6.0, 16.0, 5.0],
        uv_size: [6.0, 16.0, 5.0],
        tex: [37.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const IRON_GOLEM_TEXTURED_LEFT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.5, -3.0, -3.0],
        size: [6.0, 16.0, 5.0],
        uv_size: [6.0, 16.0, 5.0],
        tex: [60.0, 0.0],
        mirror: true,
    }];

pub(in crate::entity_models) const IRON_GOLEM_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: IRON_GOLEM_PARTS[0].pose,
        cubes: &IRON_GOLEM_TEXTURED_HEAD,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: IRON_GOLEM_PARTS[1].pose,
        cubes: &IRON_GOLEM_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: IRON_GOLEM_PARTS[2].pose,
        cubes: &IRON_GOLEM_TEXTURED_RIGHT_ARM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: IRON_GOLEM_PARTS[3].pose,
        cubes: &IRON_GOLEM_TEXTURED_LEFT_ARM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: IRON_GOLEM_PARTS[4].pose,
        cubes: &IRON_GOLEM_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: IRON_GOLEM_PARTS[5].pose,
        cubes: &IRON_GOLEM_TEXTURED_LEFT_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const SNOW_GOLEM_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.5, -7.5, -3.5],
    size: [7.0, 7.0, 7.0],
    color: SNOW_GOLEM_WHITE,
}];

pub(in crate::entity_models) const SNOW_GOLEM_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.5, 0.5, -0.5],
    size: [11.0, 1.0, 1.0],
    color: SNOW_GOLEM_WHITE,
}];

pub(in crate::entity_models) const SNOW_GOLEM_UPPER_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.5, -9.5, -4.5],
    size: [9.0, 9.0, 9.0],
    color: SNOW_GOLEM_WHITE,
}];

pub(in crate::entity_models) const SNOW_GOLEM_LOWER_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-5.5, -11.5, -5.5],
    size: [11.0, 11.0, 11.0],
    color: SNOW_GOLEM_WHITE,
}];

// Vanilla 26.1 SnowGolemModel.createBodyLayer().
pub(in crate::entity_models) const SNOW_GOLEM_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 4.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SNOW_GOLEM_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, 6.0, 1.0],
            rotation: [0.0, 0.0, 1.0],
        },
        cubes: &SNOW_GOLEM_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, 6.0, -1.0],
            rotation: [0.0, std::f32::consts::PI, -1.0],
        },
        cubes: &SNOW_GOLEM_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 13.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SNOW_GOLEM_UPPER_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 24.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SNOW_GOLEM_LOWER_BODY,
        children: &[],
    },
];

/// Vanilla `SnowGolemModel.createBodyLayer` part order: head, left arm, right arm,
/// upper body (middle snow ball), lower body. `SnowGolemModel.setupAnim` looks the
/// head and twists/orbits the upper body and arms.
pub(in crate::entity_models) const SNOW_GOLEM_HEAD_PART_INDEX: usize = 0;
pub(in crate::entity_models) const SNOW_GOLEM_LEFT_ARM_PART_INDEX: usize = 1;
pub(in crate::entity_models) const SNOW_GOLEM_RIGHT_ARM_PART_INDEX: usize = 2;
pub(in crate::entity_models) const SNOW_GOLEM_UPPER_BODY_PART_INDEX: usize = 3;

pub(in crate::entity_models) const MODEL_LAYER_SNOW_GOLEM: &str = "minecraft:snow_golem#main";

pub(in crate::entity_models) const SNOW_GOLEM_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.5, -7.5, -3.5],
        size: [7.0, 7.0, 7.0],
        uv_size: [8.0, 8.0, 8.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SNOW_GOLEM_TEXTURED_ARM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-0.5, 0.5, -0.5],
        size: [11.0, 1.0, 1.0],
        uv_size: [12.0, 2.0, 2.0],
        tex: [32.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SNOW_GOLEM_TEXTURED_UPPER_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.5, -9.5, -4.5],
        size: [9.0, 9.0, 9.0],
        uv_size: [10.0, 10.0, 10.0],
        tex: [0.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SNOW_GOLEM_TEXTURED_LOWER_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-5.5, -11.5, -5.5],
        size: [11.0, 11.0, 11.0],
        uv_size: [12.0, 12.0, 12.0],
        tex: [0.0, 36.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SNOW_GOLEM_TEXTURED_PARTS: [TexturedModelPartDesc; 5] = [
    TexturedModelPartDesc {
        pose: SNOW_GOLEM_PARTS[0].pose,
        cubes: &SNOW_GOLEM_TEXTURED_HEAD,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: SNOW_GOLEM_PARTS[1].pose,
        cubes: &SNOW_GOLEM_TEXTURED_ARM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: SNOW_GOLEM_PARTS[2].pose,
        cubes: &SNOW_GOLEM_TEXTURED_ARM,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: SNOW_GOLEM_PARTS[3].pose,
        cubes: &SNOW_GOLEM_TEXTURED_UPPER_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: SNOW_GOLEM_PARTS[4].pose,
        cubes: &SNOW_GOLEM_TEXTURED_LOWER_BODY,
        children: &[],
    },
];

/// Mutable iron golem model, mirroring vanilla `IronGolemModel`. The unified tree is zipped from the
/// baked colored ([`IRON_GOLEM_PARTS`]) and textured ([`IRON_GOLEM_TEXTURED_PARTS`]) trees: child 0 is
/// the head, child 1 the body, children 2..=5 the right/left arm and right/left leg. `setup_anim`
/// follows the head look ([`apply_head_look`]) then swings the arms and legs ([`apply_iron_golem_walk`]).
/// The attack swing and offer-flower arm pose are deferred event animations.
pub(in crate::entity_models) struct IronGolemModel {
    root: ModelPart,
}

impl IronGolemModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::root_from_descs(&IRON_GOLEM_PARTS, &IRON_GOLEM_TEXTURED_PARTS),
        }
    }
}

impl EntityModel for IronGolemModel {
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
        apply_iron_golem_walk(
            &mut self.root,
            render_state.walk_animation_pos,
            render_state.walk_animation_speed,
        );
    }
}
