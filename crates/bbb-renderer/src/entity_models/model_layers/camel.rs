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

/// Child-index path from [`ADULT_CAMEL_PARTS`] to the `head` part: body (`0`) → `head` (child `2`,
/// after the hump and tail). The baby body lists only the tail before the head, so its head is child
/// `1` ([`BABY_CAMEL_HEAD_PART_PATH`]). Used to apply the clamped head look to the nested head.
pub(in crate::entity_models) const ADULT_CAMEL_HEAD_PART_PATH: &[usize] = &[0, 2];
pub(in crate::entity_models) const BABY_CAMEL_HEAD_PART_PATH: &[usize] = &[0, 1];

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
