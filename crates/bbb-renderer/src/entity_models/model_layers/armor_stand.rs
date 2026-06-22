use super::{ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, PART_POSE_ZERO};

pub(in crate::entity_models) const ARMOR_STAND_WOOD: [f32; 4] = [0.55, 0.36, 0.19, 1.0];

pub(in crate::entity_models) const ARMOR_STAND_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -7.0, -1.0],
    size: [2.0, 7.0, 2.0],
    color: ARMOR_STAND_WOOD,
}];

pub(in crate::entity_models) const ARMOR_STAND_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-6.0, 0.0, -1.5],
    size: [12.0, 3.0, 3.0],
    color: ARMOR_STAND_WOOD,
}];

pub(in crate::entity_models) const ARMOR_STAND_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -2.0, -1.0],
    size: [2.0, 12.0, 2.0],
    color: ARMOR_STAND_WOOD,
}];

pub(in crate::entity_models) const ARMOR_STAND_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, -2.0, -1.0],
    size: [2.0, 12.0, 2.0],
    color: ARMOR_STAND_WOOD,
}];

pub(in crate::entity_models) const ARMOR_STAND_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.0],
    size: [2.0, 11.0, 2.0],
    color: ARMOR_STAND_WOOD,
}];

pub(in crate::entity_models) const ARMOR_STAND_RIGHT_BODY_STICK: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-3.0, 3.0, -1.0],
        size: [2.0, 7.0, 2.0],
        color: ARMOR_STAND_WOOD,
    }];

pub(in crate::entity_models) const ARMOR_STAND_LEFT_BODY_STICK: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [1.0, 3.0, -1.0],
        size: [2.0, 7.0, 2.0],
        color: ARMOR_STAND_WOOD,
    }];

pub(in crate::entity_models) const ARMOR_STAND_SHOULDER_STICK: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-4.0, 10.0, -1.0],
        size: [8.0, 2.0, 2.0],
        color: ARMOR_STAND_WOOD,
    }];

pub(in crate::entity_models) const ARMOR_STAND_BASE_PLATE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-6.0, 11.0, -6.0],
    size: [12.0, 1.0, 12.0],
    color: ARMOR_STAND_WOOD,
}];

// Vanilla 26.1 ArmorStandModel.createBodyLayer().
pub(in crate::entity_models) const ARMOR_STAND_PARTS: [ModelPartDesc; 10] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 1.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ARMOR_STAND_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ARMOR_STAND_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ARMOR_STAND_RIGHT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ARMOR_STAND_LEFT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ARMOR_STAND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ARMOR_STAND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ARMOR_STAND_RIGHT_BODY_STICK,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ARMOR_STAND_LEFT_BODY_STICK,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ARMOR_STAND_SHOULDER_STICK,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ARMOR_STAND_BASE_PLATE,
        children: &[],
    },
];

pub(in crate::entity_models) const SMALL_ARMOR_STAND_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.75, -5.25, -0.75],
    size: [1.5, 5.25, 1.5],
    color: ARMOR_STAND_WOOD,
}];

pub(in crate::entity_models) const SMALL_ARMOR_STAND_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, 0.0, -0.75],
    size: [6.0, 1.5, 1.5],
    color: ARMOR_STAND_WOOD,
}];

pub(in crate::entity_models) const SMALL_ARMOR_STAND_RIGHT_ARM: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-1.0, -1.0, -0.5],
        size: [1.0, 6.0, 1.0],
        color: ARMOR_STAND_WOOD,
    }];

pub(in crate::entity_models) const SMALL_ARMOR_STAND_LEFT_ARM: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [0.0, -1.0, -0.5],
        size: [1.0, 6.0, 1.0],
        color: ARMOR_STAND_WOOD,
    }];

pub(in crate::entity_models) const SMALL_ARMOR_STAND_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.5, 0.0, -0.5],
    size: [1.0, 5.5, 1.0],
    color: ARMOR_STAND_WOOD,
}];

pub(in crate::entity_models) const SMALL_ARMOR_STAND_RIGHT_BODY_STICK: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-1.5, 1.5, -0.5],
        size: [1.0, 3.5, 1.0],
        color: ARMOR_STAND_WOOD,
    }];

pub(in crate::entity_models) const SMALL_ARMOR_STAND_LEFT_BODY_STICK: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [0.5, 1.5, -0.5],
        size: [1.0, 3.5, 1.0],
        color: ARMOR_STAND_WOOD,
    }];

pub(in crate::entity_models) const SMALL_ARMOR_STAND_SHOULDER_STICK: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-2.0, 5.0, -0.5],
        size: [4.0, 1.0, 1.0],
        color: ARMOR_STAND_WOOD,
    }];

pub(in crate::entity_models) const SMALL_ARMOR_STAND_BASE_PLATE: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-3.0, 5.5, -3.0],
        size: [6.0, 0.5, 6.0],
        color: ARMOR_STAND_WOOD,
    }];

// Vanilla 26.1 ModelLayers.ARMOR_STAND_SMALL applies HumanoidModel.BABY_TRANSFORMER:
// head root parts are translated by y=16 then scaled 0.75; all other root parts
// are translated by y=24 then scaled 0.5.
pub(in crate::entity_models) const SMALL_ARMOR_STAND_PARTS: [ModelPartDesc; 10] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 12.75, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 13.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_RIGHT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, 13.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_LEFT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-0.95, 18.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.95, 18.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_RIGHT_BODY_STICK,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_LEFT_BODY_STICK,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_SHOULDER_STICK,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 18.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_BASE_PLATE,
        children: &[],
    },
];

/// The vanilla 26.1 `ArmorStandModel.createBodyLayer` `texOffs`/box for each part, in the
/// `ARMOR_STAND_PARTS` order. `uv_size` is the full-model box: the small layer is the same
/// mesh scaled by `HumanoidModel.BABY_TRANSFORMER`, which only moves vertices, so the small
/// cart samples the identical texture region as the full one.
#[derive(Clone, Copy)]
pub(in crate::entity_models) struct ArmorStandPartUv {
    pub tex: [f32; 2],
    pub uv_size: [f32; 3],
    pub mirror: bool,
}

const fn armor_stand_uv(tex: [f32; 2], uv_size: [f32; 3], mirror: bool) -> ArmorStandPartUv {
    ArmorStandPartUv {
        tex,
        uv_size,
        mirror,
    }
}

pub(in crate::entity_models) const ARMOR_STAND_PART_UVS: [ArmorStandPartUv; 10] = [
    armor_stand_uv([0.0, 0.0], [2.0, 7.0, 2.0], false), // head
    armor_stand_uv([0.0, 26.0], [12.0, 3.0, 3.0], false), // body
    armor_stand_uv([24.0, 0.0], [2.0, 12.0, 2.0], false), // right_arm
    armor_stand_uv([32.0, 16.0], [2.0, 12.0, 2.0], true), // left_arm (mirror)
    armor_stand_uv([8.0, 0.0], [2.0, 11.0, 2.0], false), // right_leg
    armor_stand_uv([40.0, 16.0], [2.0, 11.0, 2.0], true), // left_leg (mirror)
    armor_stand_uv([16.0, 0.0], [2.0, 7.0, 2.0], false), // right_body_stick
    armor_stand_uv([48.0, 16.0], [2.0, 7.0, 2.0], false), // left_body_stick
    armor_stand_uv([0.0, 48.0], [8.0, 2.0, 2.0], false), // shoulder_stick
    armor_stand_uv([0.0, 32.0], [12.0, 1.0, 12.0], false), // base_plate
];

/// Builds the textured cube for an armor-stand part: the geometry (`min`/`size`) comes from
/// the shared colored part (so the colored and textured meshes are identical), while the UV
/// source comes from the full-model `ArmorStandPartUv`.
pub(in crate::entity_models) fn armor_stand_textured_cube(
    part: &ModelPartDesc,
    uv: ArmorStandPartUv,
) -> TexturedModelCubeDesc {
    let cube = part.cubes[0];
    TexturedModelCubeDesc {
        min: cube.min,
        size: cube.size,
        uv_size: uv.uv_size,
        tex: uv.tex,
        mirror: uv.mirror,
    }
}
