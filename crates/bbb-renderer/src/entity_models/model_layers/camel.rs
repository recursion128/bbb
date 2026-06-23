use super::super::keyframe::{
    degree_vec, keyframe, pos_vec, AnimationChannel, AnimationDefinition, AnimationTarget,
    BoneAnimation, Keyframe, KeyframeInterpolation,
};
use super::{
    ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc, CAMEL_TAN,
};

pub(in crate::entity_models) const ADULT_CAMEL_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-7.5, -12.0, -23.5],
    size: [15.0, 12.0, 27.0],
    color: CAMEL_TAN,
}];

pub(in crate::entity_models) const ADULT_CAMEL_HUMP: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.5, -5.0, -5.5],
    size: [9.0, 5.0, 11.0],
    color: CAMEL_TAN,
}];

pub(in crate::entity_models) const ADULT_CAMEL_TAIL: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, 0.0, 0.0],
    size: [3.0, 14.0, 0.0],
    color: CAMEL_TAN,
}];

pub(in crate::entity_models) const ADULT_CAMEL_HEAD: [ModelCubeDesc; 3] = [
    ModelCubeDesc {
        min: [-3.5, -7.0, -15.0],
        size: [7.0, 8.0, 19.0],
        color: CAMEL_TAN,
    },
    ModelCubeDesc {
        min: [-3.5, -21.0, -15.0],
        size: [7.0, 14.0, 7.0],
        color: CAMEL_TAN,
    },
    ModelCubeDesc {
        min: [-2.5, -21.0, -21.0],
        size: [5.0, 5.0, 6.0],
        color: CAMEL_TAN,
    },
];

pub(in crate::entity_models) const ADULT_CAMEL_LEFT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.5, 0.5, -1.0],
    size: [3.0, 1.0, 2.0],
    color: CAMEL_TAN,
}];

pub(in crate::entity_models) const ADULT_CAMEL_RIGHT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.5, 0.5, -1.0],
    size: [3.0, 1.0, 2.0],
    color: CAMEL_TAN,
}];

pub(in crate::entity_models) const ADULT_CAMEL_LEFT_HIND_LEG: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-2.5, 2.0, -2.5],
        size: [5.0, 21.0, 5.0],
        color: CAMEL_TAN,
    }];

pub(in crate::entity_models) const ADULT_CAMEL_RIGHT_HIND_LEG: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-2.5, 2.0, -2.5],
        size: [5.0, 21.0, 5.0],
        color: CAMEL_TAN,
    }];

pub(in crate::entity_models) const ADULT_CAMEL_LEFT_FRONT_LEG: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-2.5, 2.0, -2.5],
        size: [5.0, 21.0, 5.0],
        color: CAMEL_TAN,
    }];

pub(in crate::entity_models) const ADULT_CAMEL_RIGHT_FRONT_LEG: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-2.5, 2.0, -2.5],
        size: [5.0, 21.0, 5.0],
        color: CAMEL_TAN,
    }];

pub(in crate::entity_models) const ADULT_CAMEL_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, -21.0, -9.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CAMEL_LEFT_EAR,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, -21.0, -9.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CAMEL_RIGHT_EAR,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_CAMEL_BODY_CHILDREN: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -12.0, -10.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CAMEL_HUMP,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -9.0, 3.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CAMEL_TAIL,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -3.0, -19.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CAMEL_HEAD,
        children: &ADULT_CAMEL_HEAD_CHILDREN,
    },
];

// Vanilla 26.1 ModelLayers.CAMEL: AdultCamelModel.createBodyLayer().
pub(in crate::entity_models) const ADULT_CAMEL_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 4.0, 9.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CAMEL_BODY,
        children: &ADULT_CAMEL_BODY_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.9, 1.0, 9.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CAMEL_LEFT_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.9, 1.0, 9.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CAMEL_RIGHT_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.9, 1.0, -10.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CAMEL_LEFT_FRONT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.9, 1.0, -10.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CAMEL_RIGHT_FRONT_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_CAMEL_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.5, -4.0, -8.0],
    size: [9.0, 8.0, 16.0],
    color: CAMEL_TAN,
}];

pub(in crate::entity_models) const BABY_CAMEL_TAIL: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, -0.5, 0.0],
    size: [3.0, 9.0, 0.0],
    color: CAMEL_TAN,
}];

pub(in crate::entity_models) const BABY_CAMEL_HEAD: [ModelCubeDesc; 3] = [
    ModelCubeDesc {
        min: [-2.5, -3.0, -7.5],
        size: [5.0, 5.0, 7.0],
        color: CAMEL_TAN,
    },
    ModelCubeDesc {
        min: [-2.5, -12.0, -7.5],
        size: [5.0, 9.0, 5.0],
        color: CAMEL_TAN,
    },
    ModelCubeDesc {
        min: [-2.5, -12.0, -10.5],
        size: [5.0, 4.0, 3.0],
        color: CAMEL_TAN,
    },
];

pub(in crate::entity_models) const BABY_CAMEL_RIGHT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -0.5, -1.0],
    size: [3.0, 1.0, 2.0],
    color: CAMEL_TAN,
}];

pub(in crate::entity_models) const BABY_CAMEL_LEFT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, -0.5, -1.0],
    size: [3.0, 1.0, 2.0],
    color: CAMEL_TAN,
}];

pub(in crate::entity_models) const BABY_CAMEL_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, -0.5, -1.5],
    size: [3.0, 13.0, 3.0],
    color: CAMEL_TAN,
}];

pub(in crate::entity_models) const BABY_CAMEL_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, -11.0, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CAMEL_RIGHT_EAR,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, -11.0, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CAMEL_LEFT_EAR,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_CAMEL_BODY_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -1.5, 8.05],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CAMEL_TAIL,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 1.0, -7.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CAMEL_HEAD,
        children: &BABY_CAMEL_HEAD_CHILDREN,
    },
];

// Vanilla 26.1 ModelLayers.CAMEL_BABY: BabyCamelModel.createBodyLayer().
pub(in crate::entity_models) const BABY_CAMEL_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 7.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CAMEL_BODY,
        children: &BABY_CAMEL_BODY_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 11.5, -5.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CAMEL_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, 11.5, -5.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CAMEL_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, 11.5, 5.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CAMEL_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 11.5, 5.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CAMEL_LEG,
        children: &[],
    },
];

/// Vanilla `CamelModel.applyHeadRotation`: the net head look clamped to `yRot ∈ [-30, 30]` and
/// `xRot ∈ [-25, 45]` (a camel turns its long neck only so far) before `head.yRot/xRot` are set from
/// the clamped degrees. Returns the clamped `(yaw, pitch)` in degrees. The transient `jumpCooldown`
/// extra-pitch boost (`45 * jumpCooldown / 55`, re-clamped to `70`) needs the un-projected
/// `jumpCooldown` render state and is deferred.
pub(in crate::entity_models) fn camel_clamped_head_look(
    head_yaw_deg: f32,
    head_pitch_deg: f32,
) -> (f32, f32) {
    (
        head_yaw_deg.clamp(-30.0, 30.0),
        head_pitch_deg.clamp(-25.0, 45.0),
    )
}

// Vanilla 26.1 `ModelLayers.CAMEL` / `CAMEL_BABY` (`CamelRenderer`,
// `CamelHuskRenderer`). The camel husk shares the adult camel's baked mesh; only the
// texture differs, so it reuses the `camel#main` layer/parts.
pub(in crate::entity_models) const MODEL_LAYER_CAMEL: &str = "minecraft:camel#main";
pub(in crate::entity_models) const MODEL_LAYER_CAMEL_BABY: &str = "minecraft:camel_baby#main";

// `AdultCamelModel.createBodyMesh` UVs, atlas 128×128. `CubeDeformation.NONE`, so every
// `uv_size` equals the geometry size. The tail is a zero-thickness (depth 0) plane.
pub(in crate::entity_models) const ADULT_CAMEL_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-7.5, -12.0, -23.5],
        size: [15.0, 12.0, 27.0],
        uv_size: [15.0, 12.0, 27.0],
        tex: [0.0, 25.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_CAMEL_TEXTURED_HUMP: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.5, -5.0, -5.5],
        size: [9.0, 5.0, 11.0],
        uv_size: [9.0, 5.0, 11.0],
        tex: [74.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_CAMEL_TEXTURED_TAIL: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.5, 0.0, 0.0],
        size: [3.0, 14.0, 0.0],
        uv_size: [3.0, 14.0, 0.0],
        tex: [122.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_CAMEL_TEXTURED_HEAD: [TexturedModelCubeDesc; 3] = [
    TexturedModelCubeDesc {
        min: [-3.5, -7.0, -15.0],
        size: [7.0, 8.0, 19.0],
        uv_size: [7.0, 8.0, 19.0],
        tex: [60.0, 24.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-3.5, -21.0, -15.0],
        size: [7.0, 14.0, 7.0],
        uv_size: [7.0, 14.0, 7.0],
        tex: [21.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-2.5, -21.0, -21.0],
        size: [5.0, 5.0, 6.0],
        uv_size: [5.0, 5.0, 6.0],
        tex: [50.0, 0.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const ADULT_CAMEL_TEXTURED_LEFT_EAR: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-0.5, 0.5, -1.0],
        size: [3.0, 1.0, 2.0],
        uv_size: [3.0, 1.0, 2.0],
        tex: [45.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_CAMEL_TEXTURED_RIGHT_EAR: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.5, 0.5, -1.0],
        size: [3.0, 1.0, 2.0],
        uv_size: [3.0, 1.0, 2.0],
        tex: [67.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_CAMEL_TEXTURED_LEFT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.5, 2.0, -2.5],
        size: [5.0, 21.0, 5.0],
        uv_size: [5.0, 21.0, 5.0],
        tex: [58.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_CAMEL_TEXTURED_RIGHT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.5, 2.0, -2.5],
        size: [5.0, 21.0, 5.0],
        uv_size: [5.0, 21.0, 5.0],
        tex: [94.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_CAMEL_TEXTURED_LEFT_FRONT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.5, 2.0, -2.5],
        size: [5.0, 21.0, 5.0],
        uv_size: [5.0, 21.0, 5.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_CAMEL_TEXTURED_RIGHT_FRONT_LEG: [TexturedModelCubeDesc;
    1] = [TexturedModelCubeDesc {
    min: [-2.5, 2.0, -2.5],
    size: [5.0, 21.0, 5.0],
    uv_size: [5.0, 21.0, 5.0],
    tex: [0.0, 26.0],
    mirror: false,
}];

pub(in crate::entity_models) const ADULT_CAMEL_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 2] = [
    TexturedModelPartDesc {
        pose: ADULT_CAMEL_HEAD_CHILDREN[0].pose,
        cubes: &ADULT_CAMEL_TEXTURED_LEFT_EAR,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_CAMEL_HEAD_CHILDREN[1].pose,
        cubes: &ADULT_CAMEL_TEXTURED_RIGHT_EAR,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_CAMEL_TEXTURED_BODY_CHILDREN: [TexturedModelPartDesc; 3] = [
    TexturedModelPartDesc {
        pose: ADULT_CAMEL_BODY_CHILDREN[0].pose,
        cubes: &ADULT_CAMEL_TEXTURED_HUMP,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_CAMEL_BODY_CHILDREN[1].pose,
        cubes: &ADULT_CAMEL_TEXTURED_TAIL,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_CAMEL_BODY_CHILDREN[2].pose,
        cubes: &ADULT_CAMEL_TEXTURED_HEAD,
        children: &ADULT_CAMEL_TEXTURED_HEAD_CHILDREN,
    },
];

pub(in crate::entity_models) const ADULT_CAMEL_TEXTURED_PARTS: [TexturedModelPartDesc; 5] = [
    TexturedModelPartDesc {
        pose: ADULT_CAMEL_PARTS[0].pose,
        cubes: &ADULT_CAMEL_TEXTURED_BODY,
        children: &ADULT_CAMEL_TEXTURED_BODY_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: ADULT_CAMEL_PARTS[1].pose,
        cubes: &ADULT_CAMEL_TEXTURED_LEFT_HIND_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_CAMEL_PARTS[2].pose,
        cubes: &ADULT_CAMEL_TEXTURED_RIGHT_HIND_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_CAMEL_PARTS[3].pose,
        cubes: &ADULT_CAMEL_TEXTURED_LEFT_FRONT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_CAMEL_PARTS[4].pose,
        cubes: &ADULT_CAMEL_TEXTURED_RIGHT_FRONT_LEG,
        children: &[],
    },
];

// `BabyCamelModel.createBodyLayer` UVs, atlas 64×64. Each leg has its own `texOffs`.
pub(in crate::entity_models) const BABY_CAMEL_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.5, -4.0, -8.0],
        size: [9.0, 8.0, 16.0],
        uv_size: [9.0, 8.0, 16.0],
        tex: [0.0, 14.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_CAMEL_TEXTURED_TAIL: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.5, -0.5, 0.0],
        size: [3.0, 9.0, 0.0],
        uv_size: [3.0, 9.0, 0.0],
        tex: [50.0, 38.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_CAMEL_TEXTURED_HEAD: [TexturedModelCubeDesc; 3] = [
    TexturedModelCubeDesc {
        min: [-2.5, -3.0, -7.5],
        size: [5.0, 5.0, 7.0],
        uv_size: [5.0, 5.0, 7.0],
        tex: [20.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-2.5, -12.0, -7.5],
        size: [5.0, 9.0, 5.0],
        uv_size: [5.0, 9.0, 5.0],
        tex: [0.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-2.5, -12.0, -10.5],
        size: [5.0, 4.0, 3.0],
        uv_size: [5.0, 4.0, 3.0],
        tex: [0.0, 14.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const BABY_CAMEL_TEXTURED_RIGHT_EAR: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.0, -0.5, -1.0],
        size: [3.0, 1.0, 2.0],
        uv_size: [3.0, 1.0, 2.0],
        tex: [37.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_CAMEL_TEXTURED_LEFT_EAR: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, -0.5, -1.0],
        size: [3.0, 1.0, 2.0],
        uv_size: [3.0, 1.0, 2.0],
        tex: [47.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_CAMEL_TEXTURED_RIGHT_FRONT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.5, -0.5, -1.5],
        size: [3.0, 13.0, 3.0],
        uv_size: [3.0, 13.0, 3.0],
        tex: [36.0, 14.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_CAMEL_TEXTURED_LEFT_FRONT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.5, -0.5, -1.5],
        size: [3.0, 13.0, 3.0],
        uv_size: [3.0, 13.0, 3.0],
        tex: [48.0, 14.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_CAMEL_TEXTURED_LEFT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.5, -0.5, -1.5],
        size: [3.0, 13.0, 3.0],
        uv_size: [3.0, 13.0, 3.0],
        tex: [12.0, 38.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_CAMEL_TEXTURED_RIGHT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.5, -0.5, -1.5],
        size: [3.0, 13.0, 3.0],
        uv_size: [3.0, 13.0, 3.0],
        tex: [0.0, 38.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_CAMEL_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 2] = [
    TexturedModelPartDesc {
        pose: BABY_CAMEL_HEAD_CHILDREN[0].pose,
        cubes: &BABY_CAMEL_TEXTURED_RIGHT_EAR,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_CAMEL_HEAD_CHILDREN[1].pose,
        cubes: &BABY_CAMEL_TEXTURED_LEFT_EAR,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_CAMEL_TEXTURED_BODY_CHILDREN: [TexturedModelPartDesc; 2] = [
    TexturedModelPartDesc {
        pose: BABY_CAMEL_BODY_CHILDREN[0].pose,
        cubes: &BABY_CAMEL_TEXTURED_TAIL,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_CAMEL_BODY_CHILDREN[1].pose,
        cubes: &BABY_CAMEL_TEXTURED_HEAD,
        children: &BABY_CAMEL_TEXTURED_HEAD_CHILDREN,
    },
];

pub(in crate::entity_models) const BABY_CAMEL_TEXTURED_PARTS: [TexturedModelPartDesc; 5] = [
    TexturedModelPartDesc {
        pose: BABY_CAMEL_PARTS[0].pose,
        cubes: &BABY_CAMEL_TEXTURED_BODY,
        children: &BABY_CAMEL_TEXTURED_BODY_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: BABY_CAMEL_PARTS[1].pose,
        cubes: &BABY_CAMEL_TEXTURED_RIGHT_FRONT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_CAMEL_PARTS[2].pose,
        cubes: &BABY_CAMEL_TEXTURED_LEFT_FRONT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_CAMEL_PARTS[3].pose,
        cubes: &BABY_CAMEL_TEXTURED_LEFT_HIND_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_CAMEL_PARTS[4].pose,
        cubes: &BABY_CAMEL_TEXTURED_RIGHT_HIND_LEG,
        children: &[],
    },
];

// ----- `CamelAnimation.CAMEL_WALK` (the adult walk; length 1.5s, looping) -----
//
// `CamelModel.setupAnim` samples it via `applyWalk(walkAnimationPos, walkAnimationSpeed, 2.0, 2.5)`.
// The `root` channel rolls the whole model (a CatmullRom z-sway applied at the entity root), the four
// legs swing (rotation + position), the `head` adds a small pitch onto the clamped look, the two ears
// flap (z-roll), and the `tail` swishes. All keyframes are CatmullRom except the two `left_hind_leg`
// closing keyframes. The baby (`CamelBabyAnimation.CAMEL_BABY_WALK`, a different cycle/topology) stays
// deferred. The adult `head`/leg/ear/tail bone names line up with the colored and textured layers.

const LINEAR: KeyframeInterpolation = KeyframeInterpolation::Linear;
const CATMULLROM: KeyframeInterpolation = KeyframeInterpolation::CatmullRom;

const CAMEL_WALK_ROOT_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 2.5), CATMULLROM),
    keyframe(1.0, degree_vec(0.0, 0.0, -2.5), CATMULLROM),
    keyframe(1.5, degree_vec(0.0, 0.0, 2.5), CATMULLROM),
];
const CAMEL_WALK_HEAD_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(2.5, 0.0, 0.0), CATMULLROM),
    keyframe(0.375, degree_vec(-2.5, 0.0, 0.0), CATMULLROM),
    keyframe(0.75, degree_vec(2.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.125, degree_vec(-2.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.5, degree_vec(2.5, 0.0, 0.0), CATMULLROM),
];
const CAMEL_WALK_RIGHT_FRONT_LEG_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(22.5, 0.0, 0.0), CATMULLROM),
    keyframe(0.75, degree_vec(-22.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.5, degree_vec(22.5, 0.0, 0.0), CATMULLROM),
];
const CAMEL_WALK_RIGHT_FRONT_LEG_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.4583, pos_vec(0.0, 4.0, 0.0), CATMULLROM),
    keyframe(0.75, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.5, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const CAMEL_WALK_LEFT_FRONT_LEG_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(-22.5, 0.0, 0.0), CATMULLROM),
    keyframe(0.75, degree_vec(22.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.5, degree_vec(-22.5, 0.0, 0.0), CATMULLROM),
];
const CAMEL_WALK_LEFT_FRONT_LEG_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.75, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.2083, pos_vec(0.0, 4.0, 0.0), CATMULLROM),
    keyframe(1.5, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const CAMEL_WALK_LEFT_HIND_LEG_ROT: [Keyframe; 4] = [
    keyframe(0.0, degree_vec(-20.4, 0.0, 0.0), CATMULLROM),
    keyframe(0.75, degree_vec(22.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.375, degree_vec(-22.5, 0.0, 0.0), LINEAR),
    keyframe(1.5, degree_vec(-20.4, 0.0, 0.0), LINEAR),
];
const CAMEL_WALK_LEFT_HIND_LEG_POS: [Keyframe; 5] = [
    keyframe(0.0, pos_vec(0.0, -0.21, 0.0), CATMULLROM),
    keyframe(0.75, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.0833, pos_vec(0.0, 4.0, 0.0), CATMULLROM),
    keyframe(1.375, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(1.5, pos_vec(0.0, -0.21, 0.0), LINEAR),
];
const CAMEL_WALK_RIGHT_HIND_LEG_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(22.5, 0.0, 0.0), CATMULLROM),
    keyframe(0.625, degree_vec(-22.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.5, degree_vec(22.5, 0.0, 0.0), CATMULLROM),
];
const CAMEL_WALK_RIGHT_HIND_LEG_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.375, pos_vec(0.0, 4.0, 0.0), CATMULLROM),
    keyframe(0.625, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.5, pos_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const CAMEL_WALK_LEFT_EAR_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.375, degree_vec(0.0, 0.0, -22.5), CATMULLROM),
    keyframe(0.75, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.125, degree_vec(0.0, 0.0, -22.5), CATMULLROM),
    keyframe(1.5, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const CAMEL_WALK_RIGHT_EAR_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(0.375, degree_vec(0.0, 0.0, 22.5), CATMULLROM),
    keyframe(0.75, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.125, degree_vec(0.0, 0.0, 22.5), CATMULLROM),
    keyframe(1.5, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
];
const CAMEL_WALK_TAIL_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(15.94102, -8.42106, 20.94102), CATMULLROM),
    keyframe(0.75, degree_vec(15.94102, 8.42106, -20.94102), CATMULLROM),
    keyframe(1.5, degree_vec(15.94102, -8.42106, 20.94102), CATMULLROM),
];

const fn rot(keyframes: &'static [Keyframe]) -> AnimationChannel {
    AnimationChannel {
        target: AnimationTarget::Rotation,
        keyframes,
    }
}
const fn pos(keyframes: &'static [Keyframe]) -> AnimationChannel {
    AnimationChannel {
        target: AnimationTarget::Position,
        keyframes,
    }
}

const CAMEL_WALK_ROOT_CHANNELS: [AnimationChannel; 1] = [rot(&CAMEL_WALK_ROOT_ROT)];
const CAMEL_WALK_HEAD_CHANNELS: [AnimationChannel; 1] = [rot(&CAMEL_WALK_HEAD_ROT)];
const CAMEL_WALK_RIGHT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_WALK_RIGHT_FRONT_LEG_ROT),
    pos(&CAMEL_WALK_RIGHT_FRONT_LEG_POS),
];
const CAMEL_WALK_LEFT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_WALK_LEFT_FRONT_LEG_ROT),
    pos(&CAMEL_WALK_LEFT_FRONT_LEG_POS),
];
const CAMEL_WALK_LEFT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_WALK_LEFT_HIND_LEG_ROT),
    pos(&CAMEL_WALK_LEFT_HIND_LEG_POS),
];
const CAMEL_WALK_RIGHT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_WALK_RIGHT_HIND_LEG_ROT),
    pos(&CAMEL_WALK_RIGHT_HIND_LEG_POS),
];
const CAMEL_WALK_LEFT_EAR_CHANNELS: [AnimationChannel; 1] = [rot(&CAMEL_WALK_LEFT_EAR_ROT)];
const CAMEL_WALK_RIGHT_EAR_CHANNELS: [AnimationChannel; 1] = [rot(&CAMEL_WALK_RIGHT_EAR_ROT)];
const CAMEL_WALK_TAIL_CHANNELS: [AnimationChannel; 1] = [rot(&CAMEL_WALK_TAIL_ROT)];

const CAMEL_WALK_BONES: [BoneAnimation; 9] = [
    BoneAnimation {
        bone: "root",
        channels: &CAMEL_WALK_ROOT_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &CAMEL_WALK_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "right_front_leg",
        channels: &CAMEL_WALK_RIGHT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_front_leg",
        channels: &CAMEL_WALK_LEFT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_hind_leg",
        channels: &CAMEL_WALK_LEFT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "right_hind_leg",
        channels: &CAMEL_WALK_RIGHT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_ear",
        channels: &CAMEL_WALK_LEFT_EAR_CHANNELS,
    },
    BoneAnimation {
        bone: "right_ear",
        channels: &CAMEL_WALK_RIGHT_EAR_CHANNELS,
    },
    BoneAnimation {
        bone: "tail",
        channels: &CAMEL_WALK_TAIL_CHANNELS,
    },
];

/// Vanilla `CamelAnimation.CAMEL_WALK`: the looping 1.5s adult walk cycle, sampled by
/// `CamelModel.setupAnim` via `applyWalk(walkAnimationPos, walkAnimationSpeed, 2.0, 2.5)`. The `root`
/// channel rolls the whole model, the `head` pitch ADDS onto the clamped look, and the legs / ears /
/// tail swing. Mostly CatmullRom (the two closing `left_hind_leg` keyframes are Linear).
pub(in crate::entity_models) const CAMEL_WALK: AnimationDefinition = AnimationDefinition {
    length_seconds: 1.5,
    looping: true,
    bones: &CAMEL_WALK_BONES,
};

/// Vanilla `CamelModel.applyWalk(..., 2.0F, 2.5F)` factors (`MAX_WALK_ANIMATION_SPEED` drives the
/// sample time, `WALK_ANIMATION_SCALE_FACTOR` the amplitude). The base `CamelModel` passes these for
/// both the adult and the baby walk.
pub(in crate::entity_models) const CAMEL_WALK_SPEED_FACTOR: f32 = 2.0;
pub(in crate::entity_models) const CAMEL_WALK_SCALE_FACTOR: f32 = 2.5;

// ----- `CamelBabyAnimation.CAMEL_BABY_WALK` (the baby walk; length 1.5s, looping) -----
//
// The baby walk animates the same nine bones as the adult plus a `body` position dip and a `head`
// position nudge (the adult had neither). The baby leg/ear order differs from the adult (see
// [`BABY_CAMEL_WALK_LAYOUT`]). Sampled like the adult via `applyWalk(..., 2.0, 2.5)`.

const CAMEL_BABY_WALK_ROOT_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 2.5), LINEAR),
    keyframe(0.75, degree_vec(0.0, 0.0, -2.5), CATMULLROM),
    keyframe(1.5, degree_vec(0.0, 0.0, 2.5), LINEAR),
];
const CAMEL_BABY_WALK_HEAD_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(2.5, 0.0, 0.0), LINEAR),
    keyframe(0.375, degree_vec(-2.5, 0.0, 0.0), CATMULLROM),
    keyframe(0.75, degree_vec(2.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.125, degree_vec(-2.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.5, degree_vec(2.5, 0.0, 0.0), LINEAR),
];
const CAMEL_BABY_WALK_HEAD_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.4583, pos_vec(0.0, 0.0, 0.1), LINEAR),
];
const CAMEL_BABY_WALK_RIGHT_FRONT_LEG_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(-22.5, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(22.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.5, degree_vec(-22.5, 0.0, 0.0), LINEAR),
];
const CAMEL_BABY_WALK_RIGHT_FRONT_LEG_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(0.075, 0.0, 0.0), LINEAR),
    keyframe(0.75, pos_vec(0.075, 0.0, 0.0), CATMULLROM),
    keyframe(1.2083, pos_vec(0.075, 4.0, 0.0), CATMULLROM),
    keyframe(1.5, pos_vec(0.075, 0.0, 0.0), LINEAR),
];
const CAMEL_BABY_WALK_LEFT_FRONT_LEG_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(22.5, 0.0, 0.0), LINEAR),
    keyframe(0.75, degree_vec(-22.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.5, degree_vec(22.5, 0.0, 0.0), LINEAR),
];
const CAMEL_BABY_WALK_LEFT_FRONT_LEG_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(-0.1, 0.0, 0.0), LINEAR),
    keyframe(0.4583, pos_vec(-0.1, 4.0, 0.0), CATMULLROM),
    keyframe(0.75, pos_vec(-0.1, 0.0, 0.0), CATMULLROM),
    keyframe(1.5, pos_vec(-0.1, 0.0, 0.0), LINEAR),
];
const CAMEL_BABY_WALK_LEFT_HIND_LEG_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(22.5, 0.0, 0.0), LINEAR),
    keyframe(0.375, degree_vec(-9.49, 0.0, 0.0), CATMULLROM),
    keyframe(0.5833, degree_vec(-17.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.2083, degree_vec(7.38, 0.0, 0.0), LINEAR),
    keyframe(1.5, degree_vec(22.5, 0.0, 0.0), LINEAR),
];
const CAMEL_BABY_WALK_LEFT_HIND_LEG_POS: [Keyframe; 4] = [
    keyframe(0.0, pos_vec(-0.1, 0.0, 0.0), LINEAR),
    keyframe(0.25, pos_vec(-0.1, 5.0, 0.0), CATMULLROM),
    keyframe(0.5833, pos_vec(-0.1, 0.0, -0.1), CATMULLROM),
    keyframe(1.5, pos_vec(-0.1, 0.0, 0.0), CATMULLROM),
];
const CAMEL_BABY_WALK_RIGHT_HIND_LEG_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(-15.83, 0.0, 0.0), CATMULLROM),
    keyframe(0.75, degree_vec(22.5, 0.0, 0.0), CATMULLROM),
    keyframe(1.0, degree_vec(-7.38, 0.0, 0.0), CATMULLROM),
    keyframe(1.25, degree_vec(-21.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.5, degree_vec(-15.83, 0.0, 0.0), CATMULLROM),
];
const CAMEL_BABY_WALK_RIGHT_HIND_LEG_POS: [Keyframe; 5] = [
    keyframe(0.0, pos_vec(0.1, 0.0, 0.0), LINEAR),
    keyframe(0.6667, pos_vec(0.1, 0.0, 0.0), CATMULLROM),
    keyframe(1.0, pos_vec(0.1, 4.0, 0.17), CATMULLROM),
    keyframe(1.2083, pos_vec(0.1, 0.0, -0.11), CATMULLROM),
    keyframe(1.5, pos_vec(0.1, 0.0, 0.0), LINEAR),
];
const CAMEL_BABY_WALK_LEFT_EAR_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.375, degree_vec(0.0, 0.0, 22.5), CATMULLROM),
    keyframe(0.75, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.125, degree_vec(0.0, 0.0, 22.5), CATMULLROM),
    keyframe(1.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const CAMEL_BABY_WALK_RIGHT_EAR_ROT: [Keyframe; 5] = [
    keyframe(0.0, degree_vec(0.0, 0.0, 0.0), LINEAR),
    keyframe(0.375, degree_vec(0.0, 0.0, -22.5), CATMULLROM),
    keyframe(0.75, degree_vec(0.0, 0.0, 0.0), CATMULLROM),
    keyframe(1.125, degree_vec(0.0, 0.0, -22.5), CATMULLROM),
    keyframe(1.5, degree_vec(0.0, 0.0, 0.0), LINEAR),
];
const CAMEL_BABY_WALK_TAIL_ROT: [Keyframe; 3] = [
    keyframe(0.0, degree_vec(15.94, -8.42, 20.94), LINEAR),
    keyframe(0.75, degree_vec(15.94, 8.42, -20.94), CATMULLROM),
    keyframe(1.5, degree_vec(15.94, -8.42, 20.94), LINEAR),
];
const CAMEL_BABY_WALK_BODY_POS: [Keyframe; 2] = [
    keyframe(0.0, pos_vec(0.0, -0.6, 0.0), LINEAR),
    keyframe(0.4583, pos_vec(0.0, -0.6, 0.0), LINEAR),
];

const CAMEL_BABY_WALK_ROOT_CHANNELS: [AnimationChannel; 1] = [rot(&CAMEL_BABY_WALK_ROOT_ROT)];
const CAMEL_BABY_WALK_HEAD_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_BABY_WALK_HEAD_ROT),
    pos(&CAMEL_BABY_WALK_HEAD_POS),
];
const CAMEL_BABY_WALK_RIGHT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_BABY_WALK_RIGHT_FRONT_LEG_ROT),
    pos(&CAMEL_BABY_WALK_RIGHT_FRONT_LEG_POS),
];
const CAMEL_BABY_WALK_LEFT_FRONT_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_BABY_WALK_LEFT_FRONT_LEG_ROT),
    pos(&CAMEL_BABY_WALK_LEFT_FRONT_LEG_POS),
];
const CAMEL_BABY_WALK_LEFT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_BABY_WALK_LEFT_HIND_LEG_ROT),
    pos(&CAMEL_BABY_WALK_LEFT_HIND_LEG_POS),
];
const CAMEL_BABY_WALK_RIGHT_HIND_LEG_CHANNELS: [AnimationChannel; 2] = [
    rot(&CAMEL_BABY_WALK_RIGHT_HIND_LEG_ROT),
    pos(&CAMEL_BABY_WALK_RIGHT_HIND_LEG_POS),
];
const CAMEL_BABY_WALK_LEFT_EAR_CHANNELS: [AnimationChannel; 1] =
    [rot(&CAMEL_BABY_WALK_LEFT_EAR_ROT)];
const CAMEL_BABY_WALK_RIGHT_EAR_CHANNELS: [AnimationChannel; 1] =
    [rot(&CAMEL_BABY_WALK_RIGHT_EAR_ROT)];
const CAMEL_BABY_WALK_TAIL_CHANNELS: [AnimationChannel; 1] = [rot(&CAMEL_BABY_WALK_TAIL_ROT)];
const CAMEL_BABY_WALK_BODY_CHANNELS: [AnimationChannel; 1] = [pos(&CAMEL_BABY_WALK_BODY_POS)];

const CAMEL_BABY_WALK_BONES: [BoneAnimation; 10] = [
    BoneAnimation {
        bone: "root",
        channels: &CAMEL_BABY_WALK_ROOT_CHANNELS,
    },
    BoneAnimation {
        bone: "head",
        channels: &CAMEL_BABY_WALK_HEAD_CHANNELS,
    },
    BoneAnimation {
        bone: "right_front_leg",
        channels: &CAMEL_BABY_WALK_RIGHT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_front_leg",
        channels: &CAMEL_BABY_WALK_LEFT_FRONT_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_hind_leg",
        channels: &CAMEL_BABY_WALK_LEFT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "right_hind_leg",
        channels: &CAMEL_BABY_WALK_RIGHT_HIND_LEG_CHANNELS,
    },
    BoneAnimation {
        bone: "left_ear",
        channels: &CAMEL_BABY_WALK_LEFT_EAR_CHANNELS,
    },
    BoneAnimation {
        bone: "right_ear",
        channels: &CAMEL_BABY_WALK_RIGHT_EAR_CHANNELS,
    },
    BoneAnimation {
        bone: "tail",
        channels: &CAMEL_BABY_WALK_TAIL_CHANNELS,
    },
    BoneAnimation {
        bone: "body",
        channels: &CAMEL_BABY_WALK_BODY_CHANNELS,
    },
];

/// Vanilla `CamelBabyAnimation.CAMEL_BABY_WALK`: the looping 1.5s baby walk cycle, sampled like the
/// adult via `applyWalk(walkAnimationPos, walkAnimationSpeed, 2.0, 2.5)`. Adds a `body` y-dip and a
/// `head` position nudge the adult lacks, and reorders the legs/ears (see [`BABY_CAMEL_WALK_LAYOUT`]).
pub(in crate::entity_models) const CAMEL_BABY_WALK: AnimationDefinition = AnimationDefinition {
    length_seconds: 1.5,
    looping: true,
    bones: &CAMEL_BABY_WALK_BONES,
};

/// The per-variant camel walk layout (which body child is the head / tail, the head-child ear order,
/// and the root-child leg order), so one hand-walk serves both the adult and the baby. The adult body
/// lists `[hump, tail, head]`; the baby `[tail, head]`.
pub(in crate::entity_models) struct CamelWalkLayout {
    pub walk: &'static AnimationDefinition,
    pub head_child: usize,
    pub tail_child: usize,
    pub ears: [(usize, &'static str); 2],
    pub legs: [(usize, &'static str); 4],
}

pub(in crate::entity_models) const ADULT_CAMEL_WALK_LAYOUT: CamelWalkLayout = CamelWalkLayout {
    walk: &CAMEL_WALK,
    head_child: 2,
    tail_child: 1,
    ears: [(0, "left_ear"), (1, "right_ear")],
    legs: [
        (1, "left_hind_leg"),
        (2, "right_hind_leg"),
        (3, "left_front_leg"),
        (4, "right_front_leg"),
    ],
};

pub(in crate::entity_models) const BABY_CAMEL_WALK_LAYOUT: CamelWalkLayout = CamelWalkLayout {
    walk: &CAMEL_BABY_WALK,
    head_child: 1,
    tail_child: 0,
    ears: [(0, "right_ear"), (1, "left_ear")],
    legs: [
        (1, "right_front_leg"),
        (2, "left_front_leg"),
        (3, "left_hind_leg"),
        (4, "right_hind_leg"),
    ],
};
