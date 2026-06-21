use super::{
    ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc,
    SHEEP_WOOL,
};

pub(in crate::entity_models) const ADULT_SHEEP_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -4.0, -6.0],
    size: [6.0, 6.0, 8.0],
    color: SHEEP_WOOL,
}];

pub(in crate::entity_models) const ADULT_SHEEP_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -10.0, -7.0],
    size: [8.0, 16.0, 6.0],
    color: SHEEP_WOOL,
}];

pub(in crate::entity_models) const ADULT_SHEEP_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: SHEEP_WOOL,
}];

// Vanilla 26.1 SheepModel.createBodyLayer().
pub(in crate::entity_models) const ADULT_SHEEP_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 6.0, -8.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_SHEEP_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 5.0, 2.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &ADULT_SHEEP_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 12.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_SHEEP_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, 12.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_SHEEP_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 12.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_SHEEP_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, 12.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_SHEEP_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_SHEEP_WOOL_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.6, -4.6, -4.6],
    size: [7.2, 7.2, 7.2],
    color: SHEEP_WOOL,
}];

pub(in crate::entity_models) const ADULT_SHEEP_WOOL_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-5.75, -11.75, -8.75],
    size: [11.5, 19.5, 9.5],
    color: SHEEP_WOOL,
}];

pub(in crate::entity_models) const ADULT_SHEEP_WOOL_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.5, -0.5, -2.5],
    size: [5.0, 7.0, 5.0],
    color: SHEEP_WOOL,
}];

// Vanilla 26.1 SheepFurModel.createFurLayer().
pub(in crate::entity_models) const ADULT_SHEEP_WOOL_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 6.0, -8.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_SHEEP_WOOL_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 5.0, 2.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &ADULT_SHEEP_WOOL_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 12.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_SHEEP_WOOL_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, 12.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_SHEEP_WOOL_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 12.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_SHEEP_WOOL_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, 12.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_SHEEP_WOOL_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_SHEEP_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.5, -4.5, -3.5],
    size: [5.0, 5.0, 5.0],
    color: SHEEP_WOOL,
}];

pub(in crate::entity_models) const BABY_SHEEP_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -2.0, -4.5],
    size: [6.0, 4.0, 9.0],
    color: SHEEP_WOOL,
}];

pub(in crate::entity_models) const BABY_SHEEP_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.0],
    size: [2.0, 5.0, 2.0],
    color: SHEEP_WOOL,
}];

// Vanilla 26.1 BabySheepModel.createBodyLayer().
pub(in crate::entity_models) const BABY_SHEEP_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 17.0, 0.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_SHEEP_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 15.5, -2.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_SHEEP_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 19.0, 3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_SHEEP_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 19.0, 3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_SHEEP_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 19.0, -2.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_SHEEP_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 19.0, -2.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_SHEEP_LEG,
        children: &[],
    },
];

/// Vanilla `SheepRenderState.headEatPositionScale` / `headEatAngleScale`, the
/// per-frame eat-grass head animation projected from `Sheep.eatAnimationTick`.
/// `SheepModel`/`SheepFurModel.setupAnim` consume these to lower and tilt the
/// head part of the base, wool, and undercoat passes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SheepHeadEatPose {
    pub position_scale: f32,
    pub angle_scale: f32,
}

impl SheepHeadEatPose {
    /// Resting head pose used when the sheep is not eating grass.
    pub const NONE: Self = Self {
        position_scale: 0.0,
        angle_scale: 0.0,
    };

    /// Vanilla `Sheep.getHeadEatPositionScale`/`getHeadEatAngleScale` projected
    /// from the canonical `eatAnimationTick` and the renderer partial tick.
    pub fn from_eat_tick(eat_animation_tick: i32, partial_tick: f32) -> Self {
        Self {
            position_scale: sheep_head_eat_position_scale(eat_animation_tick, partial_tick),
            angle_scale: sheep_head_eat_angle_scale(eat_animation_tick, partial_tick),
        }
    }

    pub(in crate::entity_models) fn is_resting(self) -> bool {
        self == Self::NONE
    }
}

/// Vanilla `Sheep.getHeadEatAngleScale` plateau angle: `(float)(Math.PI / 5)`.
const SHEEP_HEAD_EAT_PLATEAU_ANGLE: f32 = std::f32::consts::PI / 5.0;

/// Vanilla `Sheep.getHeadEatPositionScale(partialTick)`.
fn sheep_head_eat_position_scale(eat_animation_tick: i32, partial_tick: f32) -> f32 {
    if eat_animation_tick <= 0 {
        0.0
    } else if (4..=36).contains(&eat_animation_tick) {
        1.0
    } else if eat_animation_tick < 4 {
        (eat_animation_tick as f32 - partial_tick) / 4.0
    } else {
        -(eat_animation_tick as f32 - 40.0 - partial_tick) / 4.0
    }
}

/// Vanilla `Sheep.getHeadEatAngleScale(partialTick)`. The non-eating branch
/// folds in the entity look pitch (`getXRot(a) * PI/180`); head-look pitch
/// tracking is not yet projected, so the resting angle is `0.0`, matching
/// vanilla for a sheep with no pitch.
fn sheep_head_eat_angle_scale(eat_animation_tick: i32, partial_tick: f32) -> f32 {
    if eat_animation_tick > 4 && eat_animation_tick <= 36 {
        let scale = (eat_animation_tick as f32 - 4.0 - partial_tick) / 32.0;
        SHEEP_HEAD_EAT_PLATEAU_ANGLE + 0.21991149 * (scale * 28.7).sin()
    } else if eat_animation_tick > 0 {
        SHEEP_HEAD_EAT_PLATEAU_ANGLE
    } else {
        0.0
    }
}

/// Vanilla sheep models name the head part `head`. The adult body/fur layers
/// list it first; the baby body/fur layers list the body first, so the head is
/// second.
pub(in crate::entity_models) const fn sheep_head_part_index(baby: bool) -> usize {
    if baby {
        1
    } else {
        0
    }
}

/// Vanilla `SheepModel`/`SheepFurModel.setupAnim`: `head.y += headEatPositionScale
/// * 9.0 * ageScale` and `head.xRot = headEatAngleScale`. `BabySheepModel extends
/// SheepModel`, so the baby head animates with `ageScale = 0.5`
/// (`LivingEntity.getAgeScale`).
pub(in crate::entity_models) fn sheep_eaten_head_pose(
    head_pose: PartPose,
    baby: bool,
    head_eat: SheepHeadEatPose,
) -> PartPose {
    let age_scale = if baby { 0.5 } else { 1.0 };
    PartPose {
        offset: [
            head_pose.offset[0],
            head_pose.offset[1] + head_eat.position_scale * 9.0 * age_scale,
            head_pose.offset[2],
        ],
        rotation: [
            head_eat.angle_scale,
            head_pose.rotation[1],
            head_pose.rotation[2],
        ],
    }
}

pub(in crate::entity_models) const MODEL_LAYER_SHEEP: &str = "minecraft:sheep#main";
pub(in crate::entity_models) const MODEL_LAYER_SHEEP_BABY: &str = "minecraft:sheep_baby#main";
pub(in crate::entity_models) const MODEL_LAYER_SHEEP_WOOL: &str = "minecraft:sheep#wool";
pub(in crate::entity_models) const MODEL_LAYER_SHEEP_BABY_WOOL: &str = "minecraft:sheep_baby#wool";
pub(in crate::entity_models) const MODEL_LAYER_SHEEP_WOOL_UNDERCOAT: &str =
    "minecraft:sheep#wool_undercoat";
pub(in crate::entity_models) const ADULT_SHEEP_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.0, -4.0, -6.0],
        size: [6.0, 6.0, 8.0],
        uv_size: [6.0, 6.0, 8.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_SHEEP_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, -10.0, -7.0],
        size: [8.0, 16.0, 6.0],
        uv_size: [8.0, 16.0, 6.0],
        tex: [28.0, 8.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_SHEEP_TEXTURED_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, 0.0, -2.0],
        size: [4.0, 12.0, 4.0],
        uv_size: [4.0, 12.0, 4.0],
        tex: [0.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_SHEEP_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: ADULT_SHEEP_PARTS[0].pose,
        cubes: &ADULT_SHEEP_TEXTURED_HEAD,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_SHEEP_PARTS[1].pose,
        cubes: &ADULT_SHEEP_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_SHEEP_PARTS[2].pose,
        cubes: &ADULT_SHEEP_TEXTURED_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_SHEEP_PARTS[3].pose,
        cubes: &ADULT_SHEEP_TEXTURED_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_SHEEP_PARTS[4].pose,
        cubes: &ADULT_SHEEP_TEXTURED_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_SHEEP_PARTS[5].pose,
        cubes: &ADULT_SHEEP_TEXTURED_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const ADULT_SHEEP_WOOL_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.6, -4.6, -4.6],
        size: [7.2, 7.2, 7.2],
        uv_size: [6.0, 6.0, 6.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_SHEEP_WOOL_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-5.75, -11.75, -8.75],
        size: [11.5, 19.5, 9.5],
        uv_size: [8.0, 16.0, 6.0],
        tex: [28.0, 8.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_SHEEP_WOOL_TEXTURED_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.5, -0.5, -2.5],
        size: [5.0, 7.0, 5.0],
        uv_size: [4.0, 6.0, 4.0],
        tex: [0.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_SHEEP_WOOL_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: ADULT_SHEEP_WOOL_PARTS[0].pose,
        cubes: &ADULT_SHEEP_WOOL_TEXTURED_HEAD,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_SHEEP_WOOL_PARTS[1].pose,
        cubes: &ADULT_SHEEP_WOOL_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_SHEEP_WOOL_PARTS[2].pose,
        cubes: &ADULT_SHEEP_WOOL_TEXTURED_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_SHEEP_WOOL_PARTS[3].pose,
        cubes: &ADULT_SHEEP_WOOL_TEXTURED_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_SHEEP_WOOL_PARTS[4].pose,
        cubes: &ADULT_SHEEP_WOOL_TEXTURED_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_SHEEP_WOOL_PARTS[5].pose,
        cubes: &ADULT_SHEEP_WOOL_TEXTURED_LEG,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_SHEEP_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.0, -2.0, -4.5],
        size: [6.0, 4.0, 9.0],
        uv_size: [6.0, 4.0, 9.0],
        tex: [0.0, 10.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_SHEEP_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.5, -4.5, -3.5],
        size: [5.0, 5.0, 5.0],
        uv_size: [5.0, 5.0, 5.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_SHEEP_TEXTURED_RIGHT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, 5.0, 2.0],
        uv_size: [2.0, 5.0, 2.0],
        tex: [0.0, 23.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_SHEEP_TEXTURED_LEFT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, 5.0, 2.0],
        uv_size: [2.0, 5.0, 2.0],
        tex: [24.0, 12.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_SHEEP_TEXTURED_RIGHT_FRONT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, 5.0, 2.0],
        uv_size: [2.0, 5.0, 2.0],
        tex: [8.0, 23.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_SHEEP_TEXTURED_LEFT_FRONT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, 5.0, 2.0],
        uv_size: [2.0, 5.0, 2.0],
        tex: [24.0, 5.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_SHEEP_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: BABY_SHEEP_PARTS[0].pose,
        cubes: &BABY_SHEEP_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_SHEEP_PARTS[1].pose,
        cubes: &BABY_SHEEP_TEXTURED_HEAD,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_SHEEP_PARTS[2].pose,
        cubes: &BABY_SHEEP_TEXTURED_RIGHT_HIND_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_SHEEP_PARTS[3].pose,
        cubes: &BABY_SHEEP_TEXTURED_LEFT_HIND_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_SHEEP_PARTS[4].pose,
        cubes: &BABY_SHEEP_TEXTURED_RIGHT_FRONT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_SHEEP_PARTS[5].pose,
        cubes: &BABY_SHEEP_TEXTURED_LEFT_FRONT_LEG,
        children: &[],
    },
];
