use super::{
    apply_head_look, apply_humanoid_leg_swing, apply_zombie_arms_held_out, zombie_head_part_index,
    ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc,
    PART_POSE_ZERO,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_ZOMBIE: &str = "minecraft:zombie#main";
pub(in crate::entity_models) const MODEL_LAYER_ZOMBIE_BABY: &str = "minecraft:zombie_baby#main";
pub(in crate::entity_models) const MODEL_LAYER_HUSK: &str = "minecraft:husk#main";
pub(in crate::entity_models) const MODEL_LAYER_HUSK_BABY: &str = "minecraft:husk_baby#main";
pub(in crate::entity_models) const MODEL_LAYER_DROWNED: &str = "minecraft:drowned#main";
pub(in crate::entity_models) const MODEL_LAYER_DROWNED_BABY: &str = "minecraft:drowned_baby#main";
pub(in crate::entity_models) const MODEL_LAYER_ZOMBIE_VILLAGER: &str =
    "minecraft:zombie_villager#main";
pub(in crate::entity_models) const MODEL_LAYER_ZOMBIE_VILLAGER_BABY: &str =
    "minecraft:zombie_villager_baby#main";

pub(in crate::entity_models) const ZOMBIE_GREEN: [f32; 4] = [0.33, 0.62, 0.34, 1.0];
pub(in crate::entity_models) const HUSK_TAN: [f32; 4] = [0.60, 0.50, 0.31, 1.0];
pub(in crate::entity_models) const DROWNED_BLUE: [f32; 4] = [0.23, 0.48, 0.55, 1.0];
pub(in crate::entity_models) const ZOMBIE_VILLAGER_ROBE: [f32; 4] = [0.38, 0.55, 0.34, 1.0];

pub(in crate::entity_models) const ADULT_ZOMBIE_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -8.0, -4.0],
    size: [8.0, 8.0, 8.0],
    color: ZOMBIE_GREEN,
}];

pub(in crate::entity_models) const ADULT_ZOMBIE_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.5, -8.5, -4.5],
    size: [9.0, 9.0, 9.0],
    color: ZOMBIE_GREEN,
}];

pub(in crate::entity_models) const ADULT_ZOMBIE_HEAD_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_ZOMBIE_HAT,
        children: &[],
    }];

pub(in crate::entity_models) const ADULT_ZOMBIE_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -2.0],
    size: [8.0, 12.0, 4.0],
    color: ZOMBIE_GREEN,
}];

pub(in crate::entity_models) const ADULT_ZOMBIE_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: ZOMBIE_GREEN,
}];

pub(in crate::entity_models) const ADULT_ZOMBIE_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: ZOMBIE_GREEN,
}];

pub(in crate::entity_models) const ADULT_ZOMBIE_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: ZOMBIE_GREEN,
}];

// Vanilla 26.1 ModelLayers.ZOMBIE: HumanoidModel.createMesh(CubeDeformation.NONE, 0.0F).
pub(in crate::entity_models) const ADULT_ZOMBIE_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_ZOMBIE_HEAD,
        children: &ADULT_ZOMBIE_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_ZOMBIE_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_ZOMBIE_RIGHT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_ZOMBIE_LEFT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_ZOMBIE_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_ZOMBIE_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_ZOMBIE_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -2.5, -1.0],
    size: [4.0, 5.0, 2.0],
    color: ZOMBIE_GREEN,
}];

pub(in crate::entity_models) const BABY_ZOMBIE_HEAD: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-3.0, -6.25, -3.0],
        size: [6.0, 6.0, 6.0],
        color: ZOMBIE_GREEN,
    },
    // BabyZombieModel bakes CubeDeformation(0.25F) into ModelPart.Cube bounds.
    ModelCubeDesc {
        min: [-3.25, -6.4, -3.25],
        size: [6.5, 6.5, 6.5],
        color: ZOMBIE_GREEN,
    },
];

pub(in crate::entity_models) const BABY_ZOMBIE_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -0.5, -1.0],
    size: [2.0, 5.0, 2.0],
    color: ZOMBIE_GREEN,
}];

pub(in crate::entity_models) const BABY_ZOMBIE_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.0],
    size: [2.0, 4.0, 2.0],
    color: ZOMBIE_GREEN,
}];

// Vanilla 26.1 BabyZombieModel.createBodyLayer(CubeDeformation.NONE).
pub(in crate::entity_models) const BABY_ZOMBIE_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 17.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 15.25, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 15.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, 15.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, 20.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.0, 20.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_ZOMBIE_VILLAGER_HEAD: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-4.0, -10.0, -4.0],
        size: [8.0, 10.0, 8.0],
        color: ZOMBIE_VILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [-1.0, -3.0, -6.0],
        size: [2.0, 4.0, 2.0],
        color: ZOMBIE_VILLAGER_ROBE,
    },
];

pub(in crate::entity_models) const ADULT_ZOMBIE_VILLAGER_HAT: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-4.5, -10.5, -4.5],
        size: [9.0, 11.0, 9.0],
        color: ZOMBIE_VILLAGER_ROBE,
    }];

pub(in crate::entity_models) const ADULT_ZOMBIE_VILLAGER_HAT_RIM: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-8.0, -8.0, -6.0],
        size: [16.0, 16.0, 1.0],
        color: ZOMBIE_VILLAGER_ROBE,
    }];

pub(in crate::entity_models) const ADULT_ZOMBIE_VILLAGER_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-4.0, 0.0, -3.0],
        size: [8.0, 12.0, 6.0],
        color: ZOMBIE_VILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [-4.05, -0.05, -3.05],
        size: [8.1, 20.1, 6.1],
        color: ZOMBIE_VILLAGER_ROBE,
    },
];

pub(in crate::entity_models) const ADULT_ZOMBIE_VILLAGER_RIGHT_ARM: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-3.0, -2.0, -2.0],
        size: [4.0, 12.0, 4.0],
        color: ZOMBIE_VILLAGER_ROBE,
    }];

pub(in crate::entity_models) const ADULT_ZOMBIE_VILLAGER_LEFT_ARM: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-1.0, -2.0, -2.0],
        size: [4.0, 12.0, 4.0],
        color: ZOMBIE_VILLAGER_ROBE,
    }];

pub(in crate::entity_models) const ADULT_ZOMBIE_VILLAGER_LEG: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-2.0, 0.0, -2.0],
        size: [4.0, 12.0, 4.0],
        color: ZOMBIE_VILLAGER_ROBE,
    }];

pub(in crate::entity_models) const ADULT_ZOMBIE_VILLAGER_HAT_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 0.0, 0.0],
            rotation: [-std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &ADULT_ZOMBIE_VILLAGER_HAT_RIM,
        children: &[],
    }];

pub(in crate::entity_models) const ADULT_ZOMBIE_VILLAGER_HEAD_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_ZOMBIE_VILLAGER_HAT,
        children: &ADULT_ZOMBIE_VILLAGER_HAT_CHILDREN,
    }];

// Vanilla 26.1 ZombieVillagerModel.createBodyLayer().
pub(in crate::entity_models) const ADULT_ZOMBIE_VILLAGER_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_ZOMBIE_VILLAGER_HEAD,
        children: &ADULT_ZOMBIE_VILLAGER_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_ZOMBIE_VILLAGER_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_ZOMBIE_VILLAGER_RIGHT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_ZOMBIE_VILLAGER_LEFT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_ZOMBIE_VILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_ZOMBIE_VILLAGER_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_ZOMBIE_VILLAGER_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-2.0, -2.75, -1.5],
        size: [4.0, 5.0, 3.0],
        color: ZOMBIE_VILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [-2.1, -2.85, -1.6],
        size: [4.2, 6.2, 3.2],
        color: ZOMBIE_VILLAGER_ROBE,
    },
];

pub(in crate::entity_models) const BABY_ZOMBIE_VILLAGER_HEAD: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-4.0, -8.0, -3.5],
        size: [8.0, 8.0, 7.0],
        color: ZOMBIE_VILLAGER_ROBE,
    }];

pub(in crate::entity_models) const BABY_ZOMBIE_VILLAGER_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.3, -4.3, -3.8],
    size: [8.6, 8.6, 7.6],
    color: ZOMBIE_VILLAGER_ROBE,
}];

pub(in crate::entity_models) const BABY_ZOMBIE_VILLAGER_HAT_RIM: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-7.0, -0.5, -6.0],
        size: [14.0, 1.0, 12.0],
        color: ZOMBIE_VILLAGER_ROBE,
    }];

pub(in crate::entity_models) const BABY_ZOMBIE_VILLAGER_NOSE: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-1.0, -1.0, -0.5],
        size: [2.0, 2.0, 1.0],
        color: ZOMBIE_VILLAGER_ROBE,
    }];

pub(in crate::entity_models) const BABY_ZOMBIE_VILLAGER_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -0.5, -1.0],
    size: [2.0, 5.0, 2.0],
    color: ZOMBIE_VILLAGER_ROBE,
}];

pub(in crate::entity_models) const BABY_ZOMBIE_VILLAGER_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -0.5, -1.0],
    size: [2.0, 3.0, 2.0],
    color: ZOMBIE_VILLAGER_ROBE,
}];

pub(in crate::entity_models) const BABY_ZOMBIE_VILLAGER_HEAD_CHILDREN: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -4.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_VILLAGER_HAT,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -4.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_VILLAGER_HAT_RIM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -1.0, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_VILLAGER_NOSE,
        children: &[],
    },
];

// Vanilla 26.1 BabyZombieVillagerModel.createBodyLayer().
pub(in crate::entity_models) const BABY_ZOMBIE_VILLAGER_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 18.75, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_VILLAGER_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 16.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_VILLAGER_HEAD,
        children: &BABY_ZOMBIE_VILLAGER_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 15.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_VILLAGER_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, 15.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_VILLAGER_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, 21.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_VILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.0, 21.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_VILLAGER_LEG,
        children: &[],
    },
];

// ===== Textured zombie (vanilla 26.1 `zombie.png` / `zombie_baby.png`, 64x64) =====
//
// The geometry matches the colored parts above verbatim; only the UV sources differ. A
// deformed cube (the adult hat, the baby head overlay) inflates its geometry but keeps the
// base box as its `uv_size`, exactly like `CubeDeformation` in vanilla `addBox`.
const fn zombie_textured_cube(
    min: [f32; 3],
    size: [f32; 3],
    uv_size: [f32; 3],
    tex: [f32; 2],
    mirror: bool,
) -> TexturedModelCubeDesc {
    TexturedModelCubeDesc {
        min,
        size,
        uv_size,
        tex,
        mirror,
    }
}

const fn zombie_textured_part(
    offset: [f32; 3],
    cubes: &'static [TexturedModelCubeDesc],
    children: &'static [TexturedModelPartDesc],
) -> TexturedModelPartDesc {
    TexturedModelPartDesc {
        pose: PartPose {
            offset,
            rotation: [0.0, 0.0, 0.0],
        },
        cubes,
        children,
    }
}

// Adult: vanilla `HumanoidModel.createMesh` UVs. The hat is `texOffs(32, 0)` over the base
// 8x8x8 box; the left arm/leg mirror the right's `texOffs(40, 16)`/`texOffs(0, 16)`.
const ADULT_ZOMBIE_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] = [zombie_textured_cube(
    [-4.0, -8.0, -4.0],
    [8.0, 8.0, 8.0],
    [8.0, 8.0, 8.0],
    [0.0, 0.0],
    false,
)];
const ADULT_ZOMBIE_TEXTURED_HAT: [TexturedModelCubeDesc; 1] = [zombie_textured_cube(
    [-4.5, -8.5, -4.5],
    [9.0, 9.0, 9.0],
    [8.0, 8.0, 8.0],
    [32.0, 0.0],
    false,
)];
const ADULT_ZOMBIE_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 1] = [zombie_textured_part(
    [0.0, 0.0, 0.0],
    &ADULT_ZOMBIE_TEXTURED_HAT,
    &[],
)];
const ADULT_ZOMBIE_TEXTURED_BODY: [TexturedModelCubeDesc; 1] = [zombie_textured_cube(
    [-4.0, 0.0, -2.0],
    [8.0, 12.0, 4.0],
    [8.0, 12.0, 4.0],
    [16.0, 16.0],
    false,
)];
const ADULT_ZOMBIE_TEXTURED_RIGHT_ARM: [TexturedModelCubeDesc; 1] = [zombie_textured_cube(
    [-3.0, -2.0, -2.0],
    [4.0, 12.0, 4.0],
    [4.0, 12.0, 4.0],
    [40.0, 16.0],
    false,
)];
const ADULT_ZOMBIE_TEXTURED_LEFT_ARM: [TexturedModelCubeDesc; 1] = [zombie_textured_cube(
    [-1.0, -2.0, -2.0],
    [4.0, 12.0, 4.0],
    [4.0, 12.0, 4.0],
    [40.0, 16.0],
    true,
)];
const ADULT_ZOMBIE_TEXTURED_RIGHT_LEG: [TexturedModelCubeDesc; 1] = [zombie_textured_cube(
    [-2.0, 0.0, -2.0],
    [4.0, 12.0, 4.0],
    [4.0, 12.0, 4.0],
    [0.0, 16.0],
    false,
)];
const ADULT_ZOMBIE_TEXTURED_LEFT_LEG: [TexturedModelCubeDesc; 1] = [zombie_textured_cube(
    [-2.0, 0.0, -2.0],
    [4.0, 12.0, 4.0],
    [4.0, 12.0, 4.0],
    [0.0, 16.0],
    true,
)];

pub(in crate::entity_models) const ADULT_ZOMBIE_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    zombie_textured_part(
        [0.0, 0.0, 0.0],
        &ADULT_ZOMBIE_TEXTURED_HEAD,
        &ADULT_ZOMBIE_TEXTURED_HEAD_CHILDREN,
    ),
    zombie_textured_part([0.0, 0.0, 0.0], &ADULT_ZOMBIE_TEXTURED_BODY, &[]),
    zombie_textured_part([-5.0, 2.0, 0.0], &ADULT_ZOMBIE_TEXTURED_RIGHT_ARM, &[]),
    zombie_textured_part([5.0, 2.0, 0.0], &ADULT_ZOMBIE_TEXTURED_LEFT_ARM, &[]),
    zombie_textured_part([-1.9, 12.0, 0.0], &ADULT_ZOMBIE_TEXTURED_RIGHT_LEG, &[]),
    zombie_textured_part([1.9, 12.0, 0.0], &ADULT_ZOMBIE_TEXTURED_LEFT_LEG, &[]),
];

// Adult drowned: vanilla `DrownedModel.createBodyLayer` starts from `HumanoidModel.createMesh`
// (same head/hat/body/right-limb UVs as the zombie) but replaces the left arm and left leg with
// their own non-mirrored `texOffs(32, 48)` / `texOffs(16, 48)` regions (the geometry is identical
// to the humanoid limbs, only the UV source changes), so the colored geometry still matches the
// shared zombie body parts.
const ADULT_DROWNED_TEXTURED_LEFT_ARM: [TexturedModelCubeDesc; 1] = [zombie_textured_cube(
    [-1.0, -2.0, -2.0],
    [4.0, 12.0, 4.0],
    [4.0, 12.0, 4.0],
    [32.0, 48.0],
    false,
)];
const ADULT_DROWNED_TEXTURED_LEFT_LEG: [TexturedModelCubeDesc; 1] = [zombie_textured_cube(
    [-2.0, 0.0, -2.0],
    [4.0, 12.0, 4.0],
    [4.0, 12.0, 4.0],
    [16.0, 48.0],
    false,
)];

pub(in crate::entity_models) const ADULT_DROWNED_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    zombie_textured_part(
        [0.0, 0.0, 0.0],
        &ADULT_ZOMBIE_TEXTURED_HEAD,
        &ADULT_ZOMBIE_TEXTURED_HEAD_CHILDREN,
    ),
    zombie_textured_part([0.0, 0.0, 0.0], &ADULT_ZOMBIE_TEXTURED_BODY, &[]),
    zombie_textured_part([-5.0, 2.0, 0.0], &ADULT_ZOMBIE_TEXTURED_RIGHT_ARM, &[]),
    zombie_textured_part([5.0, 2.0, 0.0], &ADULT_DROWNED_TEXTURED_LEFT_ARM, &[]),
    zombie_textured_part([-1.9, 12.0, 0.0], &ADULT_ZOMBIE_TEXTURED_RIGHT_LEG, &[]),
    zombie_textured_part([1.9, 12.0, 0.0], &ADULT_DROWNED_TEXTURED_LEFT_LEG, &[]),
];

// Baby: vanilla `BabyZombieModel.createBodyLayer`. Each limb has its own `texOffs` (no
// mirroring); the head carries the base cube plus the `0.25` deformation overlay.
const BABY_ZOMBIE_TEXTURED_BODY: [TexturedModelCubeDesc; 1] = [zombie_textured_cube(
    [-2.0, -2.5, -1.0],
    [4.0, 5.0, 2.0],
    [4.0, 5.0, 2.0],
    [16.0, 16.0],
    false,
)];
const BABY_ZOMBIE_TEXTURED_HEAD: [TexturedModelCubeDesc; 2] = [
    zombie_textured_cube(
        [-3.0, -6.25, -3.0],
        [6.0, 6.0, 6.0],
        [6.0, 6.0, 6.0],
        [3.0, 3.0],
        false,
    ),
    zombie_textured_cube(
        [-3.25, -6.4, -3.25],
        [6.5, 6.5, 6.5],
        [6.0, 6.0, 6.0],
        [35.0, 3.0],
        false,
    ),
];
const BABY_ZOMBIE_TEXTURED_RIGHT_ARM: [TexturedModelCubeDesc; 1] = [zombie_textured_cube(
    [-1.0, -0.5, -1.0],
    [2.0, 5.0, 2.0],
    [2.0, 5.0, 2.0],
    [36.0, 16.0],
    false,
)];
const BABY_ZOMBIE_TEXTURED_LEFT_ARM: [TexturedModelCubeDesc; 1] = [zombie_textured_cube(
    [-1.0, -0.5, -1.0],
    [2.0, 5.0, 2.0],
    [2.0, 5.0, 2.0],
    [28.0, 16.0],
    false,
)];
const BABY_ZOMBIE_TEXTURED_RIGHT_LEG: [TexturedModelCubeDesc; 1] = [zombie_textured_cube(
    [-1.0, 0.0, -1.0],
    [2.0, 4.0, 2.0],
    [2.0, 4.0, 2.0],
    [8.0, 16.0],
    false,
)];
const BABY_ZOMBIE_TEXTURED_LEFT_LEG: [TexturedModelCubeDesc; 1] = [zombie_textured_cube(
    [-1.0, 0.0, -1.0],
    [2.0, 4.0, 2.0],
    [2.0, 4.0, 2.0],
    [0.0, 16.0],
    false,
)];

pub(in crate::entity_models) const BABY_ZOMBIE_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    zombie_textured_part([0.0, 17.5, 0.0], &BABY_ZOMBIE_TEXTURED_BODY, &[]),
    zombie_textured_part([0.0, 15.25, 0.0], &BABY_ZOMBIE_TEXTURED_HEAD, &[]),
    zombie_textured_part([-3.0, 15.5, 0.0], &BABY_ZOMBIE_TEXTURED_RIGHT_ARM, &[]),
    zombie_textured_part([3.0, 15.5, 0.0], &BABY_ZOMBIE_TEXTURED_LEFT_ARM, &[]),
    zombie_textured_part([-1.0, 20.0, 0.0], &BABY_ZOMBIE_TEXTURED_RIGHT_LEG, &[]),
    zombie_textured_part([1.0, 20.0, 0.0], &BABY_ZOMBIE_TEXTURED_LEFT_LEG, &[]),
];

// ===== Textured zombie villager (vanilla 26.1 `zombie_villager.png` / `_baby.png`, 64x64) =====
//
// `ZombieVillagerModel`/`BabyZombieVillagerModel` geometry matches the colored parts above
// verbatim; only the UV sources differ. The deformed hat/body cubes inflate their geometry but
// keep the base box as `uv_size`, exactly like `CubeDeformation` in vanilla `addBox`.
const ADULT_ZOMBIE_VILLAGER_TEXTURED_HEAD: [TexturedModelCubeDesc; 2] = [
    zombie_textured_cube(
        [-4.0, -10.0, -4.0],
        [8.0, 10.0, 8.0],
        [8.0, 10.0, 8.0],
        [0.0, 0.0],
        false,
    ),
    zombie_textured_cube(
        [-1.0, -3.0, -6.0],
        [2.0, 4.0, 2.0],
        [2.0, 4.0, 2.0],
        [24.0, 0.0],
        false,
    ),
];
const ADULT_ZOMBIE_VILLAGER_TEXTURED_HAT: [TexturedModelCubeDesc; 1] = [zombie_textured_cube(
    [-4.5, -10.5, -4.5],
    [9.0, 11.0, 9.0],
    [8.0, 10.0, 8.0],
    [32.0, 0.0],
    false,
)];
const ADULT_ZOMBIE_VILLAGER_TEXTURED_HAT_RIM: [TexturedModelCubeDesc; 1] = [zombie_textured_cube(
    [-8.0, -8.0, -6.0],
    [16.0, 16.0, 1.0],
    [16.0, 16.0, 1.0],
    [30.0, 47.0],
    false,
)];
const ADULT_ZOMBIE_VILLAGER_TEXTURED_BODY: [TexturedModelCubeDesc; 2] = [
    zombie_textured_cube(
        [-4.0, 0.0, -3.0],
        [8.0, 12.0, 6.0],
        [8.0, 12.0, 6.0],
        [16.0, 20.0],
        false,
    ),
    zombie_textured_cube(
        [-4.05, -0.05, -3.05],
        [8.1, 20.1, 6.1],
        [8.0, 20.0, 6.0],
        [0.0, 38.0],
        false,
    ),
];
const ADULT_ZOMBIE_VILLAGER_TEXTURED_RIGHT_ARM: [TexturedModelCubeDesc; 1] =
    [zombie_textured_cube(
        [-3.0, -2.0, -2.0],
        [4.0, 12.0, 4.0],
        [4.0, 12.0, 4.0],
        [44.0, 22.0],
        false,
    )];
const ADULT_ZOMBIE_VILLAGER_TEXTURED_LEFT_ARM: [TexturedModelCubeDesc; 1] = [zombie_textured_cube(
    [-1.0, -2.0, -2.0],
    [4.0, 12.0, 4.0],
    [4.0, 12.0, 4.0],
    [44.0, 22.0],
    true,
)];
const ADULT_ZOMBIE_VILLAGER_TEXTURED_RIGHT_LEG: [TexturedModelCubeDesc; 1] =
    [zombie_textured_cube(
        [-2.0, 0.0, -2.0],
        [4.0, 12.0, 4.0],
        [4.0, 12.0, 4.0],
        [0.0, 22.0],
        false,
    )];
const ADULT_ZOMBIE_VILLAGER_TEXTURED_LEFT_LEG: [TexturedModelCubeDesc; 1] = [zombie_textured_cube(
    [-2.0, 0.0, -2.0],
    [4.0, 12.0, 4.0],
    [4.0, 12.0, 4.0],
    [0.0, 22.0],
    true,
)];
const ADULT_ZOMBIE_VILLAGER_TEXTURED_HAT_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: PartPose {
            offset: [0.0, 0.0, 0.0],
            rotation: [-std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &ADULT_ZOMBIE_VILLAGER_TEXTURED_HAT_RIM,
        children: &[],
    }];
const ADULT_ZOMBIE_VILLAGER_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_ZOMBIE_VILLAGER_TEXTURED_HAT,
        children: &ADULT_ZOMBIE_VILLAGER_TEXTURED_HAT_CHILDREN,
    }];

pub(in crate::entity_models) const ADULT_ZOMBIE_VILLAGER_TEXTURED_PARTS: [TexturedModelPartDesc;
    6] = [
    zombie_textured_part(
        [0.0, 0.0, 0.0],
        &ADULT_ZOMBIE_VILLAGER_TEXTURED_HEAD,
        &ADULT_ZOMBIE_VILLAGER_TEXTURED_HEAD_CHILDREN,
    ),
    zombie_textured_part([0.0, 0.0, 0.0], &ADULT_ZOMBIE_VILLAGER_TEXTURED_BODY, &[]),
    zombie_textured_part(
        [-5.0, 2.0, 0.0],
        &ADULT_ZOMBIE_VILLAGER_TEXTURED_RIGHT_ARM,
        &[],
    ),
    zombie_textured_part(
        [5.0, 2.0, 0.0],
        &ADULT_ZOMBIE_VILLAGER_TEXTURED_LEFT_ARM,
        &[],
    ),
    zombie_textured_part(
        [-2.0, 12.0, 0.0],
        &ADULT_ZOMBIE_VILLAGER_TEXTURED_RIGHT_LEG,
        &[],
    ),
    zombie_textured_part(
        [2.0, 12.0, 0.0],
        &ADULT_ZOMBIE_VILLAGER_TEXTURED_LEFT_LEG,
        &[],
    ),
];

const BABY_ZOMBIE_VILLAGER_TEXTURED_BODY: [TexturedModelCubeDesc; 2] = [
    zombie_textured_cube(
        [-2.0, -2.75, -1.5],
        [4.0, 5.0, 3.0],
        [4.0, 5.0, 3.0],
        [0.0, 15.0],
        false,
    ),
    zombie_textured_cube(
        [-2.1, -2.85, -1.6],
        [4.2, 6.2, 3.2],
        [4.0, 6.0, 3.0],
        [16.0, 22.0],
        false,
    ),
];
const BABY_ZOMBIE_VILLAGER_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] = [zombie_textured_cube(
    [-4.0, -8.0, -3.5],
    [8.0, 8.0, 7.0],
    [8.0, 8.0, 7.0],
    [0.0, 0.0],
    false,
)];
const BABY_ZOMBIE_VILLAGER_TEXTURED_HAT: [TexturedModelCubeDesc; 1] = [zombie_textured_cube(
    [-4.3, -4.3, -3.8],
    [8.6, 8.6, 7.6],
    [8.0, 8.0, 7.0],
    [0.0, 31.0],
    false,
)];
const BABY_ZOMBIE_VILLAGER_TEXTURED_HAT_RIM: [TexturedModelCubeDesc; 1] = [zombie_textured_cube(
    [-7.0, -0.5, -6.0],
    [14.0, 1.0, 12.0],
    [14.0, 1.0, 12.0],
    [0.0, 46.0],
    false,
)];
const BABY_ZOMBIE_VILLAGER_TEXTURED_NOSE: [TexturedModelCubeDesc; 1] = [zombie_textured_cube(
    [-1.0, -1.0, -0.5],
    [2.0, 2.0, 1.0],
    [2.0, 2.0, 1.0],
    [23.0, 0.0],
    false,
)];
const BABY_ZOMBIE_VILLAGER_TEXTURED_RIGHT_ARM: [TexturedModelCubeDesc; 1] = [zombie_textured_cube(
    [-1.0, -0.5, -1.0],
    [2.0, 5.0, 2.0],
    [2.0, 5.0, 2.0],
    [24.0, 15.0],
    false,
)];
const BABY_ZOMBIE_VILLAGER_TEXTURED_LEFT_ARM: [TexturedModelCubeDesc; 1] = [zombie_textured_cube(
    [-1.0, -0.5, -1.0],
    [2.0, 5.0, 2.0],
    [2.0, 5.0, 2.0],
    [16.0, 15.0],
    false,
)];
const BABY_ZOMBIE_VILLAGER_TEXTURED_RIGHT_LEG: [TexturedModelCubeDesc; 1] = [zombie_textured_cube(
    [-1.0, -0.5, -1.0],
    [2.0, 3.0, 2.0],
    [2.0, 3.0, 2.0],
    [8.0, 23.0],
    false,
)];
const BABY_ZOMBIE_VILLAGER_TEXTURED_LEFT_LEG: [TexturedModelCubeDesc; 1] = [zombie_textured_cube(
    [-1.0, -0.5, -1.0],
    [2.0, 3.0, 2.0],
    [2.0, 3.0, 2.0],
    [0.0, 23.0],
    false,
)];
const BABY_ZOMBIE_VILLAGER_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 3] = [
    zombie_textured_part([0.0, -4.0, 0.0], &BABY_ZOMBIE_VILLAGER_TEXTURED_HAT, &[]),
    zombie_textured_part(
        [0.0, -4.5, 0.0],
        &BABY_ZOMBIE_VILLAGER_TEXTURED_HAT_RIM,
        &[],
    ),
    zombie_textured_part([0.0, -1.0, -4.0], &BABY_ZOMBIE_VILLAGER_TEXTURED_NOSE, &[]),
];

pub(in crate::entity_models) const BABY_ZOMBIE_VILLAGER_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    zombie_textured_part([0.0, 18.75, 0.0], &BABY_ZOMBIE_VILLAGER_TEXTURED_BODY, &[]),
    zombie_textured_part(
        [0.0, 16.0, 0.0],
        &BABY_ZOMBIE_VILLAGER_TEXTURED_HEAD,
        &BABY_ZOMBIE_VILLAGER_TEXTURED_HEAD_CHILDREN,
    ),
    zombie_textured_part(
        [-3.0, 15.5, 0.0],
        &BABY_ZOMBIE_VILLAGER_TEXTURED_RIGHT_ARM,
        &[],
    ),
    zombie_textured_part(
        [3.0, 15.5, 0.0],
        &BABY_ZOMBIE_VILLAGER_TEXTURED_LEFT_ARM,
        &[],
    ),
    zombie_textured_part(
        [-1.0, 21.5, 0.0],
        &BABY_ZOMBIE_VILLAGER_TEXTURED_RIGHT_LEG,
        &[],
    ),
    zombie_textured_part(
        [1.0, 21.5, 0.0],
        &BABY_ZOMBIE_VILLAGER_TEXTURED_LEFT_LEG,
        &[],
    ),
];

/// Mutable zombie model, mirroring vanilla `ZombieModel` (an `AbstractZombieModel` over `HumanoidModel`).
/// The unified tree is zipped from the baked colored ([`ADULT_ZOMBIE_PARTS`]/[`BABY_ZOMBIE_PARTS`]) and
/// textured ([`ADULT_ZOMBIE_TEXTURED_PARTS`]/[`BABY_ZOMBIE_TEXTURED_PARTS`]) trees for the selected
/// `baby` layout. `setup_anim` looks the head ([`apply_head_look`] at [`zombie_head_part_index`]), runs
/// the humanoid leg swing ([`apply_humanoid_leg_swing`]), then overrides the arms with the held-out
/// `animateZombieArms` pose ([`apply_zombie_arms_held_out`], `isAggressive`-driven).
pub(in crate::entity_models) struct ZombieModel {
    root: ModelPart,
    baby: bool,
}

impl ZombieModel {
    pub(in crate::entity_models) fn new(baby: bool) -> Self {
        let root = if baby {
            ModelPart::root_from_descs(&BABY_ZOMBIE_PARTS, &BABY_ZOMBIE_TEXTURED_PARTS)
        } else {
            ModelPart::root_from_descs(&ADULT_ZOMBIE_PARTS, &ADULT_ZOMBIE_TEXTURED_PARTS)
        };
        Self { root, baby }
    }
}

impl EntityModel for ZombieModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let render_state = &instance.render_state;
        apply_head_look(
            self.root.child_at_mut(zombie_head_part_index(self.baby)),
            render_state.head_yaw,
            render_state.head_pitch,
        );
        apply_humanoid_leg_swing(
            &mut self.root,
            render_state.walk_animation_pos,
            render_state.walk_animation_speed,
        );
        apply_zombie_arms_held_out(
            &mut self.root,
            render_state.is_aggressive,
            render_state.age_in_ticks,
        );
    }
}
