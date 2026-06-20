use super::{
    ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc,
    PART_POSE_ZERO,
};

pub(super) const CHICKEN_WHITE: [f32; 4] = [0.94, 0.94, 0.86, 1.0];
pub(super) const CHICKEN_WING: [f32; 4] = [0.82, 0.82, 0.76, 1.0];
pub(super) const CHICKEN_BEAK: [f32; 4] = [0.95, 0.62, 0.18, 1.0];
pub(super) const CHICKEN_RED: [f32; 4] = [0.86, 0.08, 0.08, 1.0];
pub(super) const CHICKEN_LEG: [f32; 4] = [0.82, 0.48, 0.12, 1.0];
pub(super) const PLAYER_BLUE: [f32; 4] = [0.22, 0.42, 0.78, 1.0];
pub(super) const HOGLIN_RED: [f32; 4] = [0.60, 0.28, 0.24, 1.0];
pub(super) const ZOGLIN_GREEN: [f32; 4] = [0.42, 0.55, 0.39, 1.0];
pub(super) const RAVAGER_GRAY: [f32; 4] = [0.44, 0.38, 0.34, 1.0];
pub(super) const VILLAGER_ROBE: [f32; 4] = [0.48, 0.34, 0.23, 1.0];
pub(super) const ILLAGER_GRAY: [f32; 4] = [0.42, 0.45, 0.48, 1.0];
pub(super) const PIG_PINK: [f32; 4] = [0.92, 0.55, 0.62, 1.0];
pub(super) const PIG_COLD_FUR: [f32; 4] = [0.82, 0.78, 0.70, 1.0];
pub(super) const SHEEP_WOOL: [f32; 4] = [0.86, 0.86, 0.80, 1.0];
pub(super) const HORSE_BROWN: [f32; 4] = [0.44, 0.27, 0.14, 1.0];
pub(super) const DONKEY_GRAY: [f32; 4] = [0.46, 0.45, 0.42, 1.0];
pub(super) const MULE_BROWN: [f32; 4] = [0.34, 0.24, 0.17, 1.0];
pub(super) const SKELETON_HORSE_BONE: [f32; 4] = [0.78, 0.78, 0.68, 1.0];
pub(super) const ZOMBIE_HORSE_GREEN: [f32; 4] = [0.32, 0.54, 0.32, 1.0];
pub(super) const CAMEL_TAN: [f32; 4] = [0.72, 0.50, 0.31, 1.0];
pub(super) const CAMEL_HUSK_BROWN: [f32; 4] = [0.42, 0.33, 0.25, 1.0];
pub(super) const LLAMA_CREAMY: [f32; 4] = [0.78, 0.65, 0.45, 1.0];
pub(super) const LLAMA_WHITE: [f32; 4] = [0.86, 0.84, 0.76, 1.0];
pub(super) const LLAMA_BROWN: [f32; 4] = [0.43, 0.27, 0.16, 1.0];
pub(super) const LLAMA_GRAY: [f32; 4] = [0.45, 0.44, 0.40, 1.0];
pub(super) const GOAT_WHITE: [f32; 4] = [0.84, 0.80, 0.70, 1.0];
pub(super) const GOAT_HORN: [f32; 4] = [0.72, 0.66, 0.54, 1.0];
pub(super) const GOAT_BEARD: [f32; 4] = [0.48, 0.42, 0.32, 1.0];
pub(super) const POLAR_BEAR_WHITE: [f32; 4] = [0.88, 0.88, 0.82, 1.0];
pub(super) const CREEPER_GREEN: [f32; 4] = [0.24, 0.68, 0.23, 1.0];
pub(super) const SPIDER_DARK: [f32; 4] = [0.16, 0.12, 0.12, 1.0];
pub(super) const ENDERMAN_DARK: [f32; 4] = [0.08, 0.06, 0.10, 1.0];
pub(super) const IRON_GOLEM_STONE: [f32; 4] = [0.74, 0.74, 0.68, 1.0];
pub(super) const SNOW_GOLEM_WHITE: [f32; 4] = [0.90, 0.92, 0.88, 1.0];
pub(super) const WITCH_ROBE: [f32; 4] = [0.28, 0.17, 0.36, 1.0];
pub(super) const WITCH_HAT_COLOR: [f32; 4] = [0.16, 0.11, 0.20, 1.0];
pub(super) const ILLAGER_ROBE: [f32; 4] = [0.38, 0.40, 0.44, 1.0];
pub(super) const ILLAGER_HAT_COLOR: [f32; 4] = [0.30, 0.31, 0.34, 1.0];
pub(super) const MINECART_GRAY: [f32; 4] = [0.34, 0.35, 0.37, 1.0];
pub(super) const BOAT_WOOD: [f32; 4] = [0.55, 0.36, 0.18, 1.0];
pub(super) const PLACEHOLDER_COLOR: [f32; 4] = [0.80, 0.20, 0.72, 1.0];

mod armor_stand;
mod boat;
mod camel;
mod chicken;
mod cow;
mod creeper;
mod enderman;
mod equine;
mod goat;
mod hoglin;
mod llama;
mod pig;
mod piglin;
mod player;
mod polar_bear;
mod ravager;
mod sheep;
mod skeleton;
mod skeleton_clothing;
mod slime;
mod spider;
mod textures;
mod wolf;
mod zombie;

pub(super) use armor_stand::*;
pub(super) use boat::*;
pub(super) use camel::*;
pub(super) use chicken::*;
pub(super) use cow::*;
pub(super) use creeper::*;
pub(super) use enderman::*;
pub(super) use equine::*;
pub(super) use goat::*;
pub(super) use hoglin::*;
pub(super) use llama::*;
pub(super) use pig::*;
pub(super) use piglin::*;
pub(super) use player::*;
pub(super) use polar_bear::*;
pub(super) use ravager::*;
pub(super) use sheep::*;
pub(super) use skeleton::*;
pub(super) use skeleton_clothing::*;
pub(super) use slime::*;
pub(super) use spider::*;
pub(super) use textures::*;
pub use textures::{
    boat_entity_texture_refs, chicken_entity_texture_refs, cow_entity_texture_refs,
    creeper_entity_texture_refs, enderman_entity_texture_refs, entity_model_texture_refs,
    pig_entity_texture_refs, player_entity_texture_refs, sheep_entity_texture_refs,
    skeleton_entity_texture_refs, slime_entity_texture_refs, spider_entity_texture_refs,
    wolf_entity_texture_refs,
};
pub(super) use wolf::*;
pub(super) use zombie::*;

pub(super) const ADULT_VILLAGER_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -10.0, -4.0],
    size: [8.0, 10.0, 8.0],
    color: VILLAGER_ROBE,
}];

pub(super) const ADULT_VILLAGER_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.51, -10.51, -4.51],
    size: [9.02, 11.02, 9.02],
    color: VILLAGER_ROBE,
}];

pub(super) const ADULT_VILLAGER_HAT_RIM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-8.0, -8.0, -6.0],
    size: [16.0, 16.0, 1.0],
    color: VILLAGER_ROBE,
}];

pub(super) const ADULT_VILLAGER_NOSE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -1.0, -6.0],
    size: [2.0, 4.0, 2.0],
    color: VILLAGER_ROBE,
}];

pub(super) const ADULT_VILLAGER_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -3.0],
    size: [8.0, 12.0, 6.0],
    color: VILLAGER_ROBE,
}];

pub(super) const ADULT_VILLAGER_JACKET: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.5, -0.5, -3.5],
    size: [9.0, 21.0, 7.0],
    color: VILLAGER_ROBE,
}];

pub(super) const ADULT_VILLAGER_ARMS: [ModelCubeDesc; 3] = [
    ModelCubeDesc {
        min: [-8.0, -2.0, -2.0],
        size: [4.0, 8.0, 4.0],
        color: VILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [4.0, -2.0, -2.0],
        size: [4.0, 8.0, 4.0],
        color: VILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [-4.0, 2.0, -2.0],
        size: [8.0, 4.0, 4.0],
        color: VILLAGER_ROBE,
    },
];

pub(super) const ADULT_VILLAGER_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: VILLAGER_ROBE,
}];

pub(super) const ADULT_VILLAGER_HAT_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [0.0, 0.0, 0.0],
        rotation: [-std::f32::consts::FRAC_PI_2, 0.0, 0.0],
    },
    cubes: &ADULT_VILLAGER_HAT_RIM,
    children: &[],
}];

pub(super) const ADULT_VILLAGER_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_VILLAGER_HAT,
        children: &ADULT_VILLAGER_HAT_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_VILLAGER_NOSE,
        children: &[],
    },
];

pub(super) const ADULT_VILLAGER_BODY_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &ADULT_VILLAGER_JACKET,
    children: &[],
}];

// Vanilla 26.1 VillagerModel.createBodyModel(), with LayerDefinitions'
// MeshTransformer.scaling(0.9375F) applied by the emitter root transform.
pub(super) const ADULT_VILLAGER_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_VILLAGER_HEAD,
        children: &ADULT_VILLAGER_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_VILLAGER_BODY,
        children: &ADULT_VILLAGER_BODY_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 3.0, -1.0],
            rotation: [-0.75, 0.0, 0.0],
        },
        cubes: &ADULT_VILLAGER_ARMS,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_VILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_VILLAGER_LEG,
        children: &[],
    },
];

pub(super) const BABY_VILLAGER_RIGHT_HAND: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-1.0, -2.4925, -1.8401],
        size: [2.0, 4.0, 2.0],
        color: VILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [5.0, -2.4925, -1.8401],
        size: [2.0, 4.0, 2.0],
        color: VILLAGER_ROBE,
    },
];

pub(super) const BABY_VILLAGER_MIDDLE_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -0.9924, -0.9825],
    size: [4.0, 2.0, 2.0],
    color: VILLAGER_ROBE,
}];

pub(super) const BABY_VILLAGER_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -0.5, -1.0],
    size: [2.0, 3.0, 2.0],
    color: VILLAGER_ROBE,
}];

pub(super) const BABY_VILLAGER_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -8.0, -3.5],
    size: [8.0, 8.0, 7.0],
    color: VILLAGER_ROBE,
}];

pub(super) const BABY_VILLAGER_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.3, -4.3, -3.8],
    size: [8.6, 8.6, 7.6],
    color: VILLAGER_ROBE,
}];

pub(super) const BABY_VILLAGER_HAT_RIM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-7.0, -0.5, -6.0],
    size: [14.0, 1.0, 12.0],
    color: VILLAGER_ROBE,
}];

pub(super) const BABY_VILLAGER_NOSE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -0.5],
    size: [2.0, 2.0, 1.0],
    color: VILLAGER_ROBE,
}];

pub(super) const BABY_VILLAGER_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -2.75, -1.5],
    size: [4.0, 5.0, 3.0],
    color: VILLAGER_ROBE,
}];

pub(super) const BABY_VILLAGER_BB_MAIN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.7, -8.2, -1.7],
    size: [4.4, 6.4, 3.4],
    color: VILLAGER_ROBE,
}];

pub(super) const BABY_VILLAGER_ARMS_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 1.4025, -0.9599],
            rotation: [-1.0472, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_RIGHT_HAND,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 0.9024, -1.8175],
            rotation: [-1.0472, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_MIDDLE_ARM,
        children: &[],
    },
];

pub(super) const BABY_VILLAGER_HEAD_CHILDREN: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -4.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_HAT,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -4.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_HAT_RIM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -2.0, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_NOSE,
        children: &[],
    },
];

// Vanilla 26.1 BabyVillagerModel.createBodyModel().
pub(super) const BABY_VILLAGER_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 17.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &[],
        children: &BABY_VILLAGER_ARMS_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, 21.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.0, 21.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 16.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_HEAD,
        children: &BABY_VILLAGER_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 18.75, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.5, 24.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_BB_MAIN,
        children: &[],
    },
];

pub(super) const IRON_GOLEM_HEAD: [ModelCubeDesc; 2] = [
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

pub(super) const IRON_GOLEM_BODY: [ModelCubeDesc; 2] = [
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

pub(super) const IRON_GOLEM_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-13.0, -2.5, -3.0],
    size: [4.0, 30.0, 6.0],
    color: IRON_GOLEM_STONE,
}];

pub(super) const IRON_GOLEM_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [9.0, -2.5, -3.0],
    size: [4.0, 30.0, 6.0],
    color: IRON_GOLEM_STONE,
}];

pub(super) const IRON_GOLEM_RIGHT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.5, -3.0, -3.0],
    size: [6.0, 16.0, 5.0],
    color: IRON_GOLEM_STONE,
}];

pub(super) const IRON_GOLEM_LEFT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.5, -3.0, -3.0],
    size: [6.0, 16.0, 5.0],
    color: IRON_GOLEM_STONE,
}];

// Vanilla 26.1 IronGolemModel.createBodyLayer().
pub(super) const IRON_GOLEM_PARTS: [ModelPartDesc; 6] = [
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

pub(super) const SNOW_GOLEM_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.5, -7.5, -3.5],
    size: [7.0, 7.0, 7.0],
    color: SNOW_GOLEM_WHITE,
}];

pub(super) const SNOW_GOLEM_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.5, 0.5, -0.5],
    size: [11.0, 1.0, 1.0],
    color: SNOW_GOLEM_WHITE,
}];

pub(super) const SNOW_GOLEM_UPPER_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.5, -9.5, -4.5],
    size: [9.0, 9.0, 9.0],
    color: SNOW_GOLEM_WHITE,
}];

pub(super) const SNOW_GOLEM_LOWER_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-5.5, -11.5, -5.5],
    size: [11.0, 11.0, 11.0],
    color: SNOW_GOLEM_WHITE,
}];

// Vanilla 26.1 SnowGolemModel.createBodyLayer().
pub(super) const SNOW_GOLEM_PARTS: [ModelPartDesc; 5] = [
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

pub(super) const WITCH_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -10.0, -4.0],
    size: [8.0, 10.0, 8.0],
    color: WITCH_ROBE,
}];

pub(super) const WITCH_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, 0.0],
    size: [10.0, 2.0, 10.0],
    color: WITCH_HAT_COLOR,
}];

pub(super) const WITCH_HAT_2: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, 0.0],
    size: [7.0, 4.0, 7.0],
    color: WITCH_HAT_COLOR,
}];

pub(super) const WITCH_HAT_3: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, 0.0],
    size: [4.0, 4.0, 4.0],
    color: WITCH_HAT_COLOR,
}];

pub(super) const WITCH_HAT_4: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.25, -0.25, -0.25],
    size: [1.5, 2.5, 1.5],
    color: WITCH_HAT_COLOR,
}];

pub(super) const WITCH_NOSE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -1.0, -6.0],
    size: [2.0, 4.0, 2.0],
    color: WITCH_ROBE,
}];

pub(super) const WITCH_MOLE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.25, 3.25, -6.5],
    size: [0.5, 0.5, 0.5],
    color: WITCH_ROBE,
}];

pub(super) const WITCH_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -3.0],
    size: [8.0, 12.0, 6.0],
    color: WITCH_ROBE,
}];

pub(super) const WITCH_JACKET: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.5, -0.5, -3.5],
    size: [9.0, 21.0, 7.0],
    color: WITCH_ROBE,
}];

pub(super) const WITCH_ARMS: [ModelCubeDesc; 3] = [
    ModelCubeDesc {
        min: [-8.0, -2.0, -2.0],
        size: [4.0, 8.0, 4.0],
        color: WITCH_ROBE,
    },
    ModelCubeDesc {
        min: [4.0, -2.0, -2.0],
        size: [4.0, 8.0, 4.0],
        color: WITCH_ROBE,
    },
    ModelCubeDesc {
        min: [-4.0, 2.0, -2.0],
        size: [8.0, 4.0, 4.0],
        color: WITCH_ROBE,
    },
];

pub(super) const WITCH_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: WITCH_ROBE,
}];

pub(super) const WITCH_HAT_3_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [1.75, -2.0, 2.0],
        rotation: [-(std::f32::consts::PI / 15.0), 0.0, 0.10471976],
    },
    cubes: &WITCH_HAT_4,
    children: &[],
}];

pub(super) const WITCH_HAT_2_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [1.75, -4.0, 2.0],
        rotation: [-0.10471976, 0.0, 0.05235988],
    },
    cubes: &WITCH_HAT_3,
    children: &WITCH_HAT_3_CHILDREN,
}];

pub(super) const WITCH_HAT_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [1.75, -4.0, 2.0],
        rotation: [-0.05235988, 0.0, 0.02617994],
    },
    cubes: &WITCH_HAT_2,
    children: &WITCH_HAT_2_CHILDREN,
}];

pub(super) const WITCH_NOSE_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [0.0, -2.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &WITCH_MOLE,
    children: &[],
}];

pub(super) const WITCH_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, -10.03125, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &WITCH_HAT,
        children: &WITCH_HAT_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &WITCH_NOSE,
        children: &WITCH_NOSE_CHILDREN,
    },
];

pub(super) const WITCH_BODY_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &WITCH_JACKET,
    children: &[],
}];

// Vanilla 26.1 WitchModel.createBodyLayer(), with LayerDefinitions'
// MeshTransformer.scaling(0.9375F) applied by the emitter root transform.
pub(super) const WITCH_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &WITCH_HEAD,
        children: &WITCH_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &WITCH_BODY,
        children: &WITCH_BODY_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 3.0, -1.0],
            rotation: [-0.75, 0.0, 0.0],
        },
        cubes: &WITCH_ARMS,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &WITCH_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &WITCH_LEG,
        children: &[],
    },
];

pub(super) const ILLAGER_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -10.0, -4.0],
    size: [8.0, 10.0, 8.0],
    color: ILLAGER_ROBE,
}];

pub(super) const ILLAGER_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.45, -10.45, -4.45],
    size: [8.9, 12.9, 8.9],
    color: ILLAGER_HAT_COLOR,
}];

pub(super) const ILLAGER_NOSE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -1.0, -6.0],
    size: [2.0, 4.0, 2.0],
    color: ILLAGER_ROBE,
}];

pub(super) const ILLAGER_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-4.0, 0.0, -3.0],
        size: [8.0, 12.0, 6.0],
        color: ILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [-4.5, -0.5, -3.5],
        size: [9.0, 21.0, 7.0],
        color: ILLAGER_ROBE,
    },
];

pub(super) const ILLAGER_CROSSED_ARMS: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-8.0, -2.0, -2.0],
        size: [4.0, 8.0, 4.0],
        color: ILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [-4.0, 2.0, -2.0],
        size: [8.0, 4.0, 4.0],
        color: ILLAGER_ROBE,
    },
];

pub(super) const ILLAGER_LEFT_SHOULDER: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [4.0, -2.0, -2.0],
    size: [4.0, 8.0, 4.0],
    color: ILLAGER_ROBE,
}];

pub(super) const ILLAGER_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: ILLAGER_ROBE,
}];

pub(super) const ILLAGER_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: ILLAGER_ROBE,
}];

pub(super) const ILLAGER_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: ILLAGER_ROBE,
}];

pub(super) const ILLAGER_HEAD_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [0.0, -2.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &ILLAGER_NOSE,
    children: &[],
}];

pub(super) const ILLAGER_HEAD_WITH_HAT_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_HAT,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_NOSE,
        children: &[],
    },
];

pub(super) const ILLAGER_CROSSED_ARM_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &ILLAGER_LEFT_SHOULDER,
    children: &[],
}];

pub(super) const ILLAGER_CROSSED_ARM_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [0.0, 3.0, -1.0],
        rotation: [-0.75, 0.0, 0.0],
    },
    cubes: &ILLAGER_CROSSED_ARMS,
    children: &ILLAGER_CROSSED_ARM_CHILDREN,
};

pub(super) const ILLAGER_RIGHT_ARM_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [-5.0, 2.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &ILLAGER_RIGHT_ARM,
    children: &[],
};

pub(super) const ILLAGER_LEFT_ARM_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [5.0, 2.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &ILLAGER_LEFT_ARM,
    children: &[],
};

// Vanilla 26.1 IllagerModel.createBodyLayer(), with LayerDefinitions'
// MeshTransformer.scaling(0.9375F) applied by the emitter root transform.
pub(super) const ILLAGER_SHARED_CROSSED_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_HEAD,
        children: &ILLAGER_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_BODY,
        children: &[],
    },
    ILLAGER_CROSSED_ARM_PART,
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
];

pub(super) const ILLAGER_SHARED_UNCROSSED_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_HEAD,
        children: &ILLAGER_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
    ILLAGER_RIGHT_ARM_PART,
    ILLAGER_LEFT_ARM_PART,
];

pub(super) const ILLAGER_ILLUSIONER_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_HEAD,
        children: &ILLAGER_HEAD_WITH_HAT_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_BODY,
        children: &[],
    },
    ILLAGER_CROSSED_ARM_PART,
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
];
