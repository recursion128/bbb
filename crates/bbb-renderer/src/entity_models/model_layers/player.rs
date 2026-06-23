use super::{
    apply_head_look, apply_humanoid_crouch, apply_humanoid_walk, player_head_part_index,
    ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc,
    PART_POSE_ZERO, PLAYER_BLUE,
};
use crate::entity_models::catalog::PlayerModelPartVisibility;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_PLAYER: &str = "minecraft:player#main";
pub(in crate::entity_models) const MODEL_LAYER_PLAYER_SLIM: &str = "minecraft:player_slim#main";

pub(in crate::entity_models) const PLAYER_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -8.0, -4.0],
    size: [8.0, 8.0, 8.0],
    color: PLAYER_BLUE,
}];

pub(in crate::entity_models) const PLAYER_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.5, -8.5, -4.5],
    size: [9.0, 9.0, 9.0],
    color: PLAYER_BLUE,
}];

pub(in crate::entity_models) const PLAYER_HEAD_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &PLAYER_HAT,
    children: &[],
}];

pub(in crate::entity_models) const PLAYER_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -2.0],
    size: [8.0, 12.0, 4.0],
    color: PLAYER_BLUE,
}];

pub(in crate::entity_models) const PLAYER_JACKET: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.25, -0.25, -2.25],
    size: [8.5, 12.5, 4.5],
    color: PLAYER_BLUE,
}];

pub(in crate::entity_models) const PLAYER_BODY_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &PLAYER_JACKET,
    children: &[],
}];

pub(in crate::entity_models) const PLAYER_WIDE_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: PLAYER_BLUE,
}];

pub(in crate::entity_models) const PLAYER_WIDE_RIGHT_SLEEVE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.25, -2.25, -2.25],
    size: [4.5, 12.5, 4.5],
    color: PLAYER_BLUE,
}];

pub(in crate::entity_models) const PLAYER_WIDE_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: PLAYER_BLUE,
}];

pub(in crate::entity_models) const PLAYER_WIDE_LEFT_SLEEVE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.25, -2.25, -2.25],
    size: [4.5, 12.5, 4.5],
    color: PLAYER_BLUE,
}];

pub(in crate::entity_models) const PLAYER_SLIM_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -2.0, -2.0],
    size: [3.0, 12.0, 4.0],
    color: PLAYER_BLUE,
}];

pub(in crate::entity_models) const PLAYER_SLIM_RIGHT_SLEEVE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.25, -2.25, -2.25],
    size: [3.5, 12.5, 4.5],
    color: PLAYER_BLUE,
}];

pub(in crate::entity_models) const PLAYER_SLIM_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, -2.0],
    size: [3.0, 12.0, 4.0],
    color: PLAYER_BLUE,
}];

pub(in crate::entity_models) const PLAYER_SLIM_LEFT_SLEEVE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.25, -2.25, -2.25],
    size: [3.5, 12.5, 4.5],
    color: PLAYER_BLUE,
}];

pub(in crate::entity_models) const PLAYER_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: PLAYER_BLUE,
}];

pub(in crate::entity_models) const PLAYER_PANTS: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.25, -0.25, -2.25],
    size: [4.5, 12.5, 4.5],
    color: PLAYER_BLUE,
}];

pub(in crate::entity_models) const PLAYER_RIGHT_PANTS_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PLAYER_PANTS,
        children: &[],
    }];

pub(in crate::entity_models) const PLAYER_LEFT_PANTS_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PLAYER_PANTS,
        children: &[],
    }];

pub(in crate::entity_models) const PLAYER_WIDE_RIGHT_ARM_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PLAYER_WIDE_RIGHT_SLEEVE,
        children: &[],
    }];

pub(in crate::entity_models) const PLAYER_WIDE_LEFT_ARM_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PLAYER_WIDE_LEFT_SLEEVE,
        children: &[],
    }];

pub(in crate::entity_models) const PLAYER_SLIM_RIGHT_ARM_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PLAYER_SLIM_RIGHT_SLEEVE,
        children: &[],
    }];

pub(in crate::entity_models) const PLAYER_SLIM_LEFT_ARM_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PLAYER_SLIM_LEFT_SLEEVE,
        children: &[],
    }];

// Vanilla 26.1 ModelLayers.PLAYER / PLAYER_SLIM:
// PlayerModel.createMesh(CubeDeformation.NONE, slim).
pub(in crate::entity_models) const PLAYER_WIDE_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PLAYER_HEAD,
        children: &PLAYER_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PLAYER_BODY,
        children: &PLAYER_BODY_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &PLAYER_WIDE_RIGHT_ARM,
        children: &PLAYER_WIDE_RIGHT_ARM_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &PLAYER_WIDE_LEFT_ARM,
        children: &PLAYER_WIDE_LEFT_ARM_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &PLAYER_LEG,
        children: &PLAYER_RIGHT_PANTS_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &PLAYER_LEG,
        children: &PLAYER_LEFT_PANTS_CHILDREN,
    },
];

pub(in crate::entity_models) const PLAYER_SLIM_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PLAYER_HEAD,
        children: &PLAYER_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PLAYER_BODY,
        children: &PLAYER_BODY_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &PLAYER_SLIM_RIGHT_ARM,
        children: &PLAYER_SLIM_RIGHT_ARM_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &PLAYER_SLIM_LEFT_ARM,
        children: &PLAYER_SLIM_LEFT_ARM_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &PLAYER_LEG,
        children: &PLAYER_RIGHT_PANTS_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &PLAYER_LEG,
        children: &PLAYER_LEFT_PANTS_CHILDREN,
    },
];

pub(in crate::entity_models) const PLAYER_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, -8.0, -4.0],
        size: [8.0, 8.0, 8.0],
        uv_size: [8.0, 8.0, 8.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_TEXTURED_HAT: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.5, -8.5, -4.5],
        size: [9.0, 9.0, 9.0],
        uv_size: [8.0, 8.0, 8.0],
        tex: [32.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PLAYER_TEXTURED_HAT,
        children: &[],
    }];

pub(in crate::entity_models) const PLAYER_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, 0.0, -2.0],
        size: [8.0, 12.0, 4.0],
        uv_size: [8.0, 12.0, 4.0],
        tex: [16.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_TEXTURED_JACKET: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.25, -0.25, -2.25],
        size: [8.5, 12.5, 4.5],
        uv_size: [8.0, 12.0, 4.0],
        tex: [16.0, 32.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_TEXTURED_BODY_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PLAYER_TEXTURED_JACKET,
        children: &[],
    }];

pub(in crate::entity_models) const PLAYER_WIDE_TEXTURED_RIGHT_ARM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.0, -2.0, -2.0],
        size: [4.0, 12.0, 4.0],
        uv_size: [4.0, 12.0, 4.0],
        tex: [40.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_WIDE_TEXTURED_RIGHT_SLEEVE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.25, -2.25, -2.25],
        size: [4.5, 12.5, 4.5],
        uv_size: [4.0, 12.0, 4.0],
        tex: [40.0, 32.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_WIDE_TEXTURED_LEFT_ARM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -2.0, -2.0],
        size: [4.0, 12.0, 4.0],
        uv_size: [4.0, 12.0, 4.0],
        tex: [32.0, 48.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_WIDE_TEXTURED_LEFT_SLEEVE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.25, -2.25, -2.25],
        size: [4.5, 12.5, 4.5],
        uv_size: [4.0, 12.0, 4.0],
        tex: [48.0, 48.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_SLIM_TEXTURED_RIGHT_ARM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, -2.0, -2.0],
        size: [3.0, 12.0, 4.0],
        uv_size: [3.0, 12.0, 4.0],
        tex: [40.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_SLIM_TEXTURED_RIGHT_SLEEVE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.25, -2.25, -2.25],
        size: [3.5, 12.5, 4.5],
        uv_size: [3.0, 12.0, 4.0],
        tex: [40.0, 32.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_SLIM_TEXTURED_LEFT_ARM: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -2.0, -2.0],
        size: [3.0, 12.0, 4.0],
        uv_size: [3.0, 12.0, 4.0],
        tex: [32.0, 48.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_SLIM_TEXTURED_LEFT_SLEEVE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.25, -2.25, -2.25],
        size: [3.5, 12.5, 4.5],
        uv_size: [3.0, 12.0, 4.0],
        tex: [48.0, 48.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_TEXTURED_RIGHT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, 0.0, -2.0],
        size: [4.0, 12.0, 4.0],
        uv_size: [4.0, 12.0, 4.0],
        tex: [0.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_TEXTURED_LEFT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, 0.0, -2.0],
        size: [4.0, 12.0, 4.0],
        uv_size: [4.0, 12.0, 4.0],
        tex: [16.0, 48.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_TEXTURED_RIGHT_PANTS: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.25, -0.25, -2.25],
        size: [4.5, 12.5, 4.5],
        uv_size: [4.0, 12.0, 4.0],
        tex: [0.0, 32.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_TEXTURED_LEFT_PANTS: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.25, -0.25, -2.25],
        size: [4.5, 12.5, 4.5],
        uv_size: [4.0, 12.0, 4.0],
        tex: [0.0, 48.0],
        mirror: false,
    }];

pub(in crate::entity_models) const PLAYER_TEXTURED_RIGHT_PANTS_CHILDREN: [TexturedModelPartDesc;
    1] = [TexturedModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &PLAYER_TEXTURED_RIGHT_PANTS,
    children: &[],
}];

pub(in crate::entity_models) const PLAYER_TEXTURED_LEFT_PANTS_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PLAYER_TEXTURED_LEFT_PANTS,
        children: &[],
    }];

pub(in crate::entity_models) const PLAYER_WIDE_TEXTURED_RIGHT_ARM_CHILDREN:
    [TexturedModelPartDesc; 1] = [TexturedModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &PLAYER_WIDE_TEXTURED_RIGHT_SLEEVE,
    children: &[],
}];

pub(in crate::entity_models) const PLAYER_WIDE_TEXTURED_LEFT_ARM_CHILDREN: [TexturedModelPartDesc;
    1] = [TexturedModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &PLAYER_WIDE_TEXTURED_LEFT_SLEEVE,
    children: &[],
}];

pub(in crate::entity_models) const PLAYER_SLIM_TEXTURED_RIGHT_ARM_CHILDREN:
    [TexturedModelPartDesc; 1] = [TexturedModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &PLAYER_SLIM_TEXTURED_RIGHT_SLEEVE,
    children: &[],
}];

pub(in crate::entity_models) const PLAYER_SLIM_TEXTURED_LEFT_ARM_CHILDREN: [TexturedModelPartDesc;
    1] = [TexturedModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &PLAYER_SLIM_TEXTURED_LEFT_SLEEVE,
    children: &[],
}];

pub(in crate::entity_models) const PLAYER_WIDE_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: PLAYER_WIDE_PARTS[0].pose,
        cubes: &PLAYER_TEXTURED_HEAD,
        children: &PLAYER_TEXTURED_HEAD_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: PLAYER_WIDE_PARTS[1].pose,
        cubes: &PLAYER_TEXTURED_BODY,
        children: &PLAYER_TEXTURED_BODY_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: PLAYER_WIDE_PARTS[2].pose,
        cubes: &PLAYER_WIDE_TEXTURED_RIGHT_ARM,
        children: &PLAYER_WIDE_TEXTURED_RIGHT_ARM_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: PLAYER_WIDE_PARTS[3].pose,
        cubes: &PLAYER_WIDE_TEXTURED_LEFT_ARM,
        children: &PLAYER_WIDE_TEXTURED_LEFT_ARM_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: PLAYER_WIDE_PARTS[4].pose,
        cubes: &PLAYER_TEXTURED_RIGHT_LEG,
        children: &PLAYER_TEXTURED_RIGHT_PANTS_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: PLAYER_WIDE_PARTS[5].pose,
        cubes: &PLAYER_TEXTURED_LEFT_LEG,
        children: &PLAYER_TEXTURED_LEFT_PANTS_CHILDREN,
    },
];

pub(in crate::entity_models) const PLAYER_SLIM_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: PLAYER_SLIM_PARTS[0].pose,
        cubes: &PLAYER_TEXTURED_HEAD,
        children: &PLAYER_TEXTURED_HEAD_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: PLAYER_SLIM_PARTS[1].pose,
        cubes: &PLAYER_TEXTURED_BODY,
        children: &PLAYER_TEXTURED_BODY_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: PLAYER_SLIM_PARTS[2].pose,
        cubes: &PLAYER_SLIM_TEXTURED_RIGHT_ARM,
        children: &PLAYER_SLIM_TEXTURED_RIGHT_ARM_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: PLAYER_SLIM_PARTS[3].pose,
        cubes: &PLAYER_SLIM_TEXTURED_LEFT_ARM,
        children: &PLAYER_SLIM_TEXTURED_LEFT_ARM_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: PLAYER_SLIM_PARTS[4].pose,
        cubes: &PLAYER_TEXTURED_RIGHT_LEG,
        children: &PLAYER_TEXTURED_RIGHT_PANTS_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: PLAYER_SLIM_PARTS[5].pose,
        cubes: &PLAYER_TEXTURED_LEFT_LEG,
        children: &PLAYER_TEXTURED_LEFT_PANTS_CHILDREN,
    },
];

/// Selects the colored ([`PLAYER_WIDE_PARTS`]/[`PLAYER_SLIM_PARTS`]) and textured
/// ([`PLAYER_WIDE_TEXTURED_PARTS`]/[`PLAYER_SLIM_TEXTURED_PARTS`]) const trees for a player by the
/// `slim` arm model, zipped into the unified tree by [`PlayerModel::new`].
pub(in crate::entity_models) fn player_part_trees(
    slim: bool,
) -> (&'static [ModelPartDesc], &'static [TexturedModelPartDesc]) {
    if slim {
        (&PLAYER_SLIM_PARTS, &PLAYER_SLIM_TEXTURED_PARTS)
    } else {
        (&PLAYER_WIDE_PARTS, &PLAYER_WIDE_TEXTURED_PARTS)
    }
}

/// Mutable player model, mirroring vanilla `PlayerModel extends HumanoidModel`. The unified tree is
/// zipped from the colored and textured const trees selected by the `slim`/wide arm model
/// ([`player_part_trees`]); each of the six base parts (head, body, arms, legs) carries one skin
/// overlay child (hat/jacket/sleeve/pants). `setup_anim` looks the head, runs the inherited
/// `HumanoidModel` walk swing + idle arm bob ([`apply_humanoid_walk`]), then the crouch sneaking
/// pose ([`apply_humanoid_crouch`]). The held-item/attack arm poses, swim, and the elytra defer.
pub(in crate::entity_models) struct PlayerModel {
    root: ModelPart,
}

impl PlayerModel {
    pub(in crate::entity_models) fn new(slim: bool) -> Self {
        let (colored, textured) = player_part_trees(slim);
        Self {
            root: ModelPart::root_from_descs(colored, textured),
        }
    }

    /// Toggles the six skin-customization overlay children (hat/jacket/right & left sleeve/right &
    /// left pants), which the base parts `[0..6]` each carry as their single child, by the player's
    /// `PlayerModelPartVisibility`. The textured path calls this after [`EntityModel::prepare`]; the
    /// colored fallback leaves every overlay visible (vanilla renders untextured players whole).
    pub(in crate::entity_models) fn apply_part_visibility(
        &mut self,
        parts: PlayerModelPartVisibility,
    ) {
        let overlays = [
            parts.hat,
            parts.jacket,
            parts.right_sleeve,
            parts.left_sleeve,
            parts.right_pants,
            parts.left_pants,
        ];
        for (index, visible) in overlays.into_iter().enumerate() {
            self.root.child_at_mut(index).child_at_mut(0).visible = visible;
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
            self.root.child_at_mut(player_head_part_index()),
            render_state.head_yaw,
            render_state.head_pitch,
        );
        apply_humanoid_walk(
            &mut self.root,
            render_state.walk_animation_pos,
            render_state.walk_animation_speed,
            render_state.age_in_ticks,
        );
        if render_state.is_crouching {
            apply_humanoid_crouch(&mut self.root);
        }
    }
}
