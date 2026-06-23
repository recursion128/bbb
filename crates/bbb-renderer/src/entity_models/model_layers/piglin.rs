use super::{
    apply_head_look, apply_humanoid_leg_swing, apply_humanoid_walk, piglin_ear_flap_pose,
    piglin_head_part_index, ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc,
    TexturedModelPartDesc, PART_POSE_ZERO, PIGLIN_ADULT_EAR_ANGLE, PIGLIN_BABY_EAR_ANGLE,
};
use crate::entity_models::catalog::PiglinModelFamily;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_PIGLIN: &str = "minecraft:piglin#main";
pub(in crate::entity_models) const MODEL_LAYER_PIGLIN_BABY: &str = "minecraft:piglin_baby#main";
pub(in crate::entity_models) const MODEL_LAYER_PIGLIN_BRUTE: &str = "minecraft:piglin_brute#main";
pub(in crate::entity_models) const MODEL_LAYER_ZOMBIFIED_PIGLIN: &str =
    "minecraft:zombified_piglin#main";
pub(in crate::entity_models) const MODEL_LAYER_ZOMBIFIED_PIGLIN_BABY: &str =
    "minecraft:zombified_piglin_baby#main";

pub(in crate::entity_models) const PIGLIN_SKIN: [f32; 4] = [0.74, 0.44, 0.36, 1.0];
pub(in crate::entity_models) const PIGLIN_BRUTE_SKIN: [f32; 4] = [0.58, 0.35, 0.29, 1.0];
pub(in crate::entity_models) const ZOMBIFIED_PIGLIN_SKIN: [f32; 4] = [0.46, 0.62, 0.42, 1.0];

pub(in crate::entity_models) const ADULT_PIGLIN_HEAD: [ModelCubeDesc; 4] = [
    ModelCubeDesc {
        min: [-5.0, -8.0, -4.0],
        size: [10.0, 8.0, 8.0],
        color: PIGLIN_SKIN,
    },
    ModelCubeDesc {
        min: [-2.0, -4.0, -5.0],
        size: [4.0, 4.0, 1.0],
        color: PIGLIN_SKIN,
    },
    ModelCubeDesc {
        min: [2.0, -2.0, -5.0],
        size: [1.0, 2.0, 1.0],
        color: PIGLIN_SKIN,
    },
    ModelCubeDesc {
        min: [-3.0, -2.0, -5.0],
        size: [1.0, 2.0, 1.0],
        color: PIGLIN_SKIN,
    },
];

pub(in crate::entity_models) const ADULT_PIGLIN_LEFT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, -2.0],
    size: [1.0, 5.0, 4.0],
    color: PIGLIN_SKIN,
}];

pub(in crate::entity_models) const ADULT_PIGLIN_RIGHT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -2.0],
    size: [1.0, 5.0, 4.0],
    color: PIGLIN_SKIN,
}];

pub(in crate::entity_models) const ADULT_PIGLIN_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [4.5, -6.0, 0.0],
            rotation: [0.0, 0.0, -std::f32::consts::FRAC_PI_6],
        },
        cubes: &ADULT_PIGLIN_LEFT_EAR,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.5, -6.0, 0.0],
            rotation: [0.0, 0.0, std::f32::consts::FRAC_PI_6],
        },
        cubes: &ADULT_PIGLIN_RIGHT_EAR,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_PIGLIN_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -2.0],
    size: [8.0, 12.0, 4.0],
    color: PIGLIN_SKIN,
}];

pub(in crate::entity_models) const ADULT_PIGLIN_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: PIGLIN_SKIN,
}];

pub(in crate::entity_models) const ADULT_PIGLIN_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: PIGLIN_SKIN,
}];

pub(in crate::entity_models) const ADULT_PIGLIN_RIGHT_SLEEVE: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-3.25, -2.25, -2.25],
        size: [4.5, 12.5, 4.5],
        color: PIGLIN_SKIN,
    }];

pub(in crate::entity_models) const ADULT_PIGLIN_LEFT_SLEEVE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.25, -2.25, -2.25],
    size: [4.5, 12.5, 4.5],
    color: PIGLIN_SKIN,
}];

pub(in crate::entity_models) const ADULT_PIGLIN_RIGHT_ARM_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_PIGLIN_RIGHT_SLEEVE,
        children: &[],
    }];

pub(in crate::entity_models) const ADULT_PIGLIN_LEFT_ARM_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_PIGLIN_LEFT_SLEEVE,
        children: &[],
    }];

pub(in crate::entity_models) const ADULT_PIGLIN_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: PIGLIN_SKIN,
}];

pub(in crate::entity_models) const ADULT_PIGLIN_PANTS: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.25, -0.25, -2.25],
    size: [4.5, 12.5, 4.5],
    color: PIGLIN_SKIN,
}];

pub(in crate::entity_models) const ADULT_PIGLIN_LEG_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_PIGLIN_PANTS,
        children: &[],
    }];

// Vanilla 26.1 AdultPiglinModel.createBodyLayer().
pub(in crate::entity_models) const ADULT_PIGLIN_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_PIGLIN_HEAD,
        children: &ADULT_PIGLIN_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_PIGLIN_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_PIGLIN_RIGHT_ARM,
        children: &ADULT_PIGLIN_RIGHT_ARM_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_PIGLIN_LEFT_ARM,
        children: &ADULT_PIGLIN_LEFT_ARM_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_PIGLIN_LEG,
        children: &ADULT_PIGLIN_LEG_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_PIGLIN_LEG,
        children: &ADULT_PIGLIN_LEG_CHILDREN,
    },
];

pub(in crate::entity_models) const BABY_PIGLIN_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -3.0, -1.0],
    size: [6.0, 5.0, 3.0],
    color: PIGLIN_SKIN,
}];

pub(in crate::entity_models) const BABY_PIGLIN_HEAD: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-1.5, -3.0, -4.5],
        size: [3.0, 3.0, 1.0],
        color: PIGLIN_SKIN,
    },
    ModelCubeDesc {
        min: [-4.5, -6.0, -3.5],
        size: [9.0, 6.0, 7.0],
        color: PIGLIN_SKIN,
    },
];

pub(in crate::entity_models) const BABY_PIGLIN_LEFT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.5, -3.0, -2.0],
    size: [1.0, 6.0, 4.0],
    color: PIGLIN_SKIN,
}];

pub(in crate::entity_models) const BABY_PIGLIN_RIGHT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.5, -3.0, -2.0],
    size: [1.0, 6.0, 4.0],
    color: PIGLIN_SKIN,
}];

pub(in crate::entity_models) const BABY_PIGLIN_HAT_CHILD: ModelPartDesc = ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &[],
    children: &[],
};

pub(in crate::entity_models) const BABY_PIGLIN_LEFT_EAR_ROTATED_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PartPose {
            offset: [1.0, 1.75, 0.0],
            rotation: [0.0, 0.0, -0.6109],
        },
        cubes: &BABY_PIGLIN_LEFT_EAR,
        children: &[],
    }];

pub(in crate::entity_models) const BABY_PIGLIN_RIGHT_EAR_ROTATED_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, 1.75, 0.0],
            rotation: [0.0, 0.0, 0.6109],
        },
        cubes: &BABY_PIGLIN_RIGHT_EAR,
        children: &[],
    }];

pub(in crate::entity_models) const BABY_PIGLIN_HEAD_CHILDREN: [ModelPartDesc; 3] = [
    BABY_PIGLIN_HAT_CHILD,
    ModelPartDesc {
        pose: PartPose {
            offset: [4.2, -4.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &[],
        children: &BABY_PIGLIN_LEFT_EAR_ROTATED_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.2, -4.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &[],
        children: &BABY_PIGLIN_RIGHT_EAR_ROTATED_CHILDREN,
    },
];

pub(in crate::entity_models) const BABY_PIGLIN_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.5],
    size: [2.0, 5.0, 3.0],
    color: PIGLIN_SKIN,
}];

pub(in crate::entity_models) const BABY_PIGLIN_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.5],
    size: [2.0, 5.0, 3.0],
    color: PIGLIN_SKIN,
}];

pub(in crate::entity_models) const BABY_PIGLIN_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, 0.0, -1.5],
    size: [3.0, 4.0, 3.0],
    color: PIGLIN_SKIN,
}];

// Vanilla 26.1 BabyPiglinModel.createBodyLayer().
pub(in crate::entity_models) const BABY_PIGLIN_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 18.0, -0.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIGLIN_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 15.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIGLIN_HEAD,
        children: &BABY_PIGLIN_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 15.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIGLIN_LEFT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 15.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIGLIN_RIGHT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.5, 20.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIGLIN_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.5, 20.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIGLIN_LEG,
        children: &[],
    },
];

// ===== Textured piglin (vanilla 26.1, 64x64) =====
//
// `AbstractPiglinModel.addHead` UVs for the snouted head + ears, the `texOffs(16, 16)` body, and
// the shared `PlayerModel.createMesh` wide arm/sleeve/leg/pants UVs (the piglin clears the body
// `jacket` but keeps the arm sleeves and leg pants). All five families share this geometry
// (`AdultZombifiedPiglinModel`/`BabyZombifiedPiglinModel` forward to the piglin layers, and the
// brute reuses the adult layer); only the texture and the held-vs-swung arms differ. The deformed
// sleeve/pants cubes inflate their geometry but keep the base box as `uv_size`. The geometry
// (min/size) matches the colored cubes above so both render paths share the same mesh.
const fn piglin_textured_cube(
    min: [f32; 3],
    size: [f32; 3],
    uv_size: [f32; 3],
    tex: [f32; 2],
) -> TexturedModelCubeDesc {
    TexturedModelCubeDesc {
        min,
        size,
        uv_size,
        tex,
        mirror: false,
    }
}

const fn piglin_textured_part(
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

const ADULT_PIGLIN_TEXTURED_HEAD: [TexturedModelCubeDesc; 4] = [
    piglin_textured_cube(
        [-5.0, -8.0, -4.0],
        [10.0, 8.0, 8.0],
        [10.0, 8.0, 8.0],
        [0.0, 0.0],
    ),
    piglin_textured_cube(
        [-2.0, -4.0, -5.0],
        [4.0, 4.0, 1.0],
        [4.0, 4.0, 1.0],
        [31.0, 1.0],
    ),
    piglin_textured_cube(
        [2.0, -2.0, -5.0],
        [1.0, 2.0, 1.0],
        [1.0, 2.0, 1.0],
        [2.0, 4.0],
    ),
    piglin_textured_cube(
        [-3.0, -2.0, -5.0],
        [1.0, 2.0, 1.0],
        [1.0, 2.0, 1.0],
        [2.0, 0.0],
    ),
];
const ADULT_PIGLIN_TEXTURED_LEFT_EAR: [TexturedModelCubeDesc; 1] = [piglin_textured_cube(
    [0.0, 0.0, -2.0],
    [1.0, 5.0, 4.0],
    [1.0, 5.0, 4.0],
    [51.0, 6.0],
)];
const ADULT_PIGLIN_TEXTURED_RIGHT_EAR: [TexturedModelCubeDesc; 1] = [piglin_textured_cube(
    [-1.0, 0.0, -2.0],
    [1.0, 5.0, 4.0],
    [1.0, 5.0, 4.0],
    [39.0, 6.0],
)];
const ADULT_PIGLIN_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 2] = [
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [4.5, -6.0, 0.0],
            rotation: [0.0, 0.0, -std::f32::consts::FRAC_PI_6],
        },
        cubes: &ADULT_PIGLIN_TEXTURED_LEFT_EAR,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [-4.5, -6.0, 0.0],
            rotation: [0.0, 0.0, std::f32::consts::FRAC_PI_6],
        },
        cubes: &ADULT_PIGLIN_TEXTURED_RIGHT_EAR,
        children: &[],
    },
];
const ADULT_PIGLIN_TEXTURED_BODY: [TexturedModelCubeDesc; 1] = [piglin_textured_cube(
    [-4.0, 0.0, -2.0],
    [8.0, 12.0, 4.0],
    [8.0, 12.0, 4.0],
    [16.0, 16.0],
)];
const ADULT_PIGLIN_TEXTURED_RIGHT_ARM: [TexturedModelCubeDesc; 1] = [piglin_textured_cube(
    [-3.0, -2.0, -2.0],
    [4.0, 12.0, 4.0],
    [4.0, 12.0, 4.0],
    [40.0, 16.0],
)];
const ADULT_PIGLIN_TEXTURED_RIGHT_SLEEVE: [TexturedModelCubeDesc; 1] = [piglin_textured_cube(
    [-3.25, -2.25, -2.25],
    [4.5, 12.5, 4.5],
    [4.0, 12.0, 4.0],
    [40.0, 32.0],
)];
const ADULT_PIGLIN_TEXTURED_LEFT_ARM: [TexturedModelCubeDesc; 1] = [piglin_textured_cube(
    [-1.0, -2.0, -2.0],
    [4.0, 12.0, 4.0],
    [4.0, 12.0, 4.0],
    [32.0, 48.0],
)];
const ADULT_PIGLIN_TEXTURED_LEFT_SLEEVE: [TexturedModelCubeDesc; 1] = [piglin_textured_cube(
    [-1.25, -2.25, -2.25],
    [4.5, 12.5, 4.5],
    [4.0, 12.0, 4.0],
    [48.0, 48.0],
)];
const ADULT_PIGLIN_TEXTURED_RIGHT_LEG: [TexturedModelCubeDesc; 1] = [piglin_textured_cube(
    [-2.0, 0.0, -2.0],
    [4.0, 12.0, 4.0],
    [4.0, 12.0, 4.0],
    [0.0, 16.0],
)];
const ADULT_PIGLIN_TEXTURED_RIGHT_PANTS: [TexturedModelCubeDesc; 1] = [piglin_textured_cube(
    [-2.25, -0.25, -2.25],
    [4.5, 12.5, 4.5],
    [4.0, 12.0, 4.0],
    [0.0, 32.0],
)];
const ADULT_PIGLIN_TEXTURED_LEFT_LEG: [TexturedModelCubeDesc; 1] = [piglin_textured_cube(
    [-2.0, 0.0, -2.0],
    [4.0, 12.0, 4.0],
    [4.0, 12.0, 4.0],
    [16.0, 48.0],
)];
const ADULT_PIGLIN_TEXTURED_LEFT_PANTS: [TexturedModelCubeDesc; 1] = [piglin_textured_cube(
    [-2.25, -0.25, -2.25],
    [4.5, 12.5, 4.5],
    [4.0, 12.0, 4.0],
    [0.0, 48.0],
)];
const ADULT_PIGLIN_TEXTURED_RIGHT_ARM_CHILDREN: [TexturedModelPartDesc; 1] =
    [piglin_textured_part(
        [0.0, 0.0, 0.0],
        &ADULT_PIGLIN_TEXTURED_RIGHT_SLEEVE,
        &[],
    )];
const ADULT_PIGLIN_TEXTURED_LEFT_ARM_CHILDREN: [TexturedModelPartDesc; 1] = [piglin_textured_part(
    [0.0, 0.0, 0.0],
    &ADULT_PIGLIN_TEXTURED_LEFT_SLEEVE,
    &[],
)];
const ADULT_PIGLIN_TEXTURED_RIGHT_LEG_CHILDREN: [TexturedModelPartDesc; 1] =
    [piglin_textured_part(
        [0.0, 0.0, 0.0],
        &ADULT_PIGLIN_TEXTURED_RIGHT_PANTS,
        &[],
    )];
const ADULT_PIGLIN_TEXTURED_LEFT_LEG_CHILDREN: [TexturedModelPartDesc; 1] = [piglin_textured_part(
    [0.0, 0.0, 0.0],
    &ADULT_PIGLIN_TEXTURED_LEFT_PANTS,
    &[],
)];

pub(in crate::entity_models) const ADULT_PIGLIN_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    piglin_textured_part(
        [0.0, 0.0, 0.0],
        &ADULT_PIGLIN_TEXTURED_HEAD,
        &ADULT_PIGLIN_TEXTURED_HEAD_CHILDREN,
    ),
    piglin_textured_part([0.0, 0.0, 0.0], &ADULT_PIGLIN_TEXTURED_BODY, &[]),
    piglin_textured_part(
        [-5.0, 2.0, 0.0],
        &ADULT_PIGLIN_TEXTURED_RIGHT_ARM,
        &ADULT_PIGLIN_TEXTURED_RIGHT_ARM_CHILDREN,
    ),
    piglin_textured_part(
        [5.0, 2.0, 0.0],
        &ADULT_PIGLIN_TEXTURED_LEFT_ARM,
        &ADULT_PIGLIN_TEXTURED_LEFT_ARM_CHILDREN,
    ),
    piglin_textured_part(
        [-1.9, 12.0, 0.0],
        &ADULT_PIGLIN_TEXTURED_RIGHT_LEG,
        &ADULT_PIGLIN_TEXTURED_RIGHT_LEG_CHILDREN,
    ),
    piglin_textured_part(
        [1.9, 12.0, 0.0],
        &ADULT_PIGLIN_TEXTURED_LEFT_LEG,
        &ADULT_PIGLIN_TEXTURED_LEFT_LEG_CHILDREN,
    ),
];

const BABY_PIGLIN_TEXTURED_HEAD: [TexturedModelCubeDesc; 2] = [
    piglin_textured_cube(
        [-1.5, -3.0, -4.5],
        [3.0, 3.0, 1.0],
        [3.0, 3.0, 1.0],
        [21.0, 30.0],
    ),
    piglin_textured_cube(
        [-4.5, -6.0, -3.5],
        [9.0, 6.0, 7.0],
        [9.0, 6.0, 7.0],
        [0.0, 0.0],
    ),
];
const BABY_PIGLIN_TEXTURED_LEFT_EAR: [TexturedModelCubeDesc; 1] = [piglin_textured_cube(
    [-0.5, -3.0, -2.0],
    [1.0, 6.0, 4.0],
    [1.0, 6.0, 4.0],
    [0.0, 21.0],
)];
const BABY_PIGLIN_TEXTURED_RIGHT_EAR: [TexturedModelCubeDesc; 1] = [piglin_textured_cube(
    [-0.5, -3.0, -2.0],
    [1.0, 6.0, 4.0],
    [1.0, 6.0, 4.0],
    [18.0, 13.0],
)];
const BABY_PIGLIN_TEXTURED_LEFT_EAR_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: PartPose {
            offset: [1.0, 1.75, 0.0],
            rotation: [0.0, 0.0, -0.6109],
        },
        cubes: &BABY_PIGLIN_TEXTURED_LEFT_EAR,
        children: &[],
    }];
const BABY_PIGLIN_TEXTURED_RIGHT_EAR_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: PartPose {
            offset: [-1.0, 1.75, 0.0],
            rotation: [0.0, 0.0, 0.6109],
        },
        cubes: &BABY_PIGLIN_TEXTURED_RIGHT_EAR,
        children: &[],
    }];
const BABY_PIGLIN_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 3] = [
    piglin_textured_part([0.0, 0.0, 0.0], &[], &[]),
    piglin_textured_part(
        [4.2, -4.0, 0.0],
        &[],
        &BABY_PIGLIN_TEXTURED_LEFT_EAR_CHILDREN,
    ),
    piglin_textured_part(
        [-4.2, -4.0, 0.0],
        &[],
        &BABY_PIGLIN_TEXTURED_RIGHT_EAR_CHILDREN,
    ),
];
const BABY_PIGLIN_TEXTURED_BODY: [TexturedModelCubeDesc; 1] = [piglin_textured_cube(
    [-3.0, -3.0, -1.0],
    [6.0, 5.0, 3.0],
    [6.0, 5.0, 3.0],
    [0.0, 13.0],
)];
const BABY_PIGLIN_TEXTURED_LEFT_ARM: [TexturedModelCubeDesc; 1] = [piglin_textured_cube(
    [-1.0, 0.0, -1.5],
    [2.0, 5.0, 3.0],
    [2.0, 5.0, 3.0],
    [28.0, 13.0],
)];
const BABY_PIGLIN_TEXTURED_RIGHT_ARM: [TexturedModelCubeDesc; 1] = [piglin_textured_cube(
    [-1.0, 0.0, -1.5],
    [2.0, 5.0, 3.0],
    [2.0, 5.0, 3.0],
    [10.0, 30.0],
)];
const BABY_PIGLIN_TEXTURED_RIGHT_LEG: [TexturedModelCubeDesc; 1] = [piglin_textured_cube(
    [-1.5, 0.0, -1.5],
    [3.0, 4.0, 3.0],
    [3.0, 4.0, 3.0],
    [22.0, 23.0],
)];
const BABY_PIGLIN_TEXTURED_LEFT_LEG: [TexturedModelCubeDesc; 1] = [piglin_textured_cube(
    [-1.5, 0.0, -1.5],
    [3.0, 4.0, 3.0],
    [3.0, 4.0, 3.0],
    [10.0, 23.0],
)];

pub(in crate::entity_models) const BABY_PIGLIN_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    piglin_textured_part([0.0, 18.0, -0.5], &BABY_PIGLIN_TEXTURED_BODY, &[]),
    piglin_textured_part(
        [0.0, 15.0, 0.0],
        &BABY_PIGLIN_TEXTURED_HEAD,
        &BABY_PIGLIN_TEXTURED_HEAD_CHILDREN,
    ),
    piglin_textured_part([4.0, 15.0, 0.0], &BABY_PIGLIN_TEXTURED_LEFT_ARM, &[]),
    piglin_textured_part([-4.0, 15.0, 0.0], &BABY_PIGLIN_TEXTURED_RIGHT_ARM, &[]),
    piglin_textured_part([-1.5, 20.0, 0.0], &BABY_PIGLIN_TEXTURED_RIGHT_LEG, &[]),
    piglin_textured_part([1.5, 20.0, 0.0], &BABY_PIGLIN_TEXTURED_LEFT_LEG, &[]),
];

/// Whether a piglin renders the baby layer. The brute reuses the adult model even as a baby, so only
/// the (zombified) piglin uses the smaller baby layout. Drives the part tree, head index, ear
/// children, and default ear angle.
fn piglin_baby_layout(family: PiglinModelFamily, baby: bool) -> bool {
    baby && family != PiglinModelFamily::PiglinBrute
}

/// Ear child indices `(left, right)` under the piglin head part. The adult/brute layout lists the two
/// ears directly at `[0, 1]`; the baby layout lists the hat at `0` and the ear holders at `[1, 2]`.
fn piglin_ear_child_indices(baby_layout: bool) -> (usize, usize) {
    if baby_layout {
        (1, 2)
    } else {
        (0, 1)
    }
}

/// `AbstractPiglinModel.getDefaultEarAngleInDegrees()` (in radians): `5°` baby, `30°` adult/brute.
fn piglin_default_ear_angle(baby_layout: bool) -> f32 {
    if baby_layout {
        PIGLIN_BABY_EAR_ANGLE
    } else {
        PIGLIN_ADULT_EAR_ANGLE
    }
}

/// Selects the colored ([`ADULT_PIGLIN_PARTS`]/[`BABY_PIGLIN_PARTS`]) and textured
/// ([`ADULT_PIGLIN_TEXTURED_PARTS`]/[`BABY_PIGLIN_TEXTURED_PARTS`]) const trees for a piglin by its
/// [`piglin_baby_layout`], zipped into the unified tree by [`PiglinModel::new`].
pub(in crate::entity_models) fn piglin_part_trees(
    family: PiglinModelFamily,
    baby: bool,
) -> (&'static [ModelPartDesc], &'static [TexturedModelPartDesc]) {
    if piglin_baby_layout(family, baby) {
        (&BABY_PIGLIN_PARTS, &BABY_PIGLIN_TEXTURED_PARTS)
    } else {
        (&ADULT_PIGLIN_PARTS, &ADULT_PIGLIN_TEXTURED_PARTS)
    }
}

/// Mutable piglin model, mirroring vanilla `AbstractPiglinModel extends HumanoidModel` (the piglin,
/// piglin brute, and zombified piglin). The unified tree is selected by `family`/`baby`
/// ([`piglin_part_trees`]). `setup_anim` runs `super.setupAnim` — the head look ([`apply_head_look`])
/// and the humanoid walk (leg + arm swing/bob, [`apply_humanoid_walk`]) — except the zombified piglin
/// keeps its arms at rest (the held-out `animateZombieArms` pose defers), so it swings only the legs
/// ([`apply_humanoid_leg_swing`]); then it always flaps the two ears ([`piglin_ear_flap_pose`], head
/// children). The family recolor/texture is supplied by the caller; the dance/attack/crossbow/admire
/// arm poses and held items defer.
pub(in crate::entity_models) struct PiglinModel {
    root: ModelPart,
    family: PiglinModelFamily,
    baby_layout: bool,
}

impl PiglinModel {
    pub(in crate::entity_models) fn new(family: PiglinModelFamily, baby: bool) -> Self {
        let (colored, textured) = piglin_part_trees(family, baby);
        Self {
            root: ModelPart::root_from_descs(colored, textured),
            family,
            baby_layout: piglin_baby_layout(family, baby),
        }
    }
}

impl EntityModel for PiglinModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let render_state = &instance.render_state;
        let limb_swing = render_state.walk_animation_pos;
        let limb_swing_amount = render_state.walk_animation_speed;
        let age_in_ticks = render_state.age_in_ticks;
        let head_index = piglin_head_part_index(self.baby_layout);
        apply_head_look(
            self.root.child_at_mut(head_index),
            render_state.head_yaw,
            render_state.head_pitch,
        );
        if self.family == PiglinModelFamily::ZombifiedPiglin {
            // The zombified piglin's held-out arms (deferred) replace the inherited swing, so only the
            // legs swing.
            apply_humanoid_leg_swing(&mut self.root, limb_swing, limb_swing_amount);
        } else {
            apply_humanoid_walk(&mut self.root, limb_swing, limb_swing_amount, age_in_ticks);
        }
        // Flap the two ears (head children) every frame.
        let (left_ear, right_ear) = piglin_ear_child_indices(self.baby_layout);
        let default_ear_angle = piglin_default_ear_angle(self.baby_layout);
        let head = self.root.child_at_mut(head_index);
        let left = head.child_at_mut(left_ear);
        left.pose = piglin_ear_flap_pose(
            left.pose,
            true,
            default_ear_angle,
            age_in_ticks,
            limb_swing,
            limb_swing_amount,
        );
        let right = head.child_at_mut(right_ear);
        right.pose = piglin_ear_flap_pose(
            right.pose,
            false,
            default_ear_angle,
            age_in_ticks,
            limb_swing,
            limb_swing_amount,
        );
    }
}
