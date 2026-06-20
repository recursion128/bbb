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
pub(super) const PIGLIN_SKIN: [f32; 4] = [0.74, 0.44, 0.36, 1.0];
pub(super) const PIGLIN_BRUTE_SKIN: [f32; 4] = [0.58, 0.35, 0.29, 1.0];
pub(super) const ZOMBIFIED_PIGLIN_SKIN: [f32; 4] = [0.46, 0.62, 0.42, 1.0];
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
mod chicken;
mod cow;
mod creeper;
mod enderman;
mod pig;
mod player;
mod sheep;
mod skeleton;
mod slime;
mod spider;
mod textures;
mod wolf;
mod zombie;

pub(super) use armor_stand::*;
pub(super) use boat::*;
pub(super) use chicken::*;
pub(super) use cow::*;
pub(super) use creeper::*;
pub(super) use enderman::*;
pub(super) use pig::*;
pub(super) use player::*;
pub(super) use sheep::*;
pub(super) use skeleton::*;
pub(super) use slime::*;
pub(super) use spider::*;
pub(super) use textures::*;
pub use textures::{
    boat_entity_texture_refs, chicken_entity_texture_refs, cow_entity_texture_refs,
    creeper_entity_texture_refs, enderman_entity_texture_refs, entity_model_texture_refs,
    pig_entity_texture_refs, player_entity_texture_refs, sheep_entity_texture_refs,
    spider_entity_texture_refs, wolf_entity_texture_refs,
};
pub(super) use wolf::*;
pub(super) use zombie::*;

pub(super) const ADULT_PIGLIN_HEAD: [ModelCubeDesc; 4] = [
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

pub(super) const ADULT_PIGLIN_LEFT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, -2.0],
    size: [1.0, 5.0, 4.0],
    color: PIGLIN_SKIN,
}];

pub(super) const ADULT_PIGLIN_RIGHT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -2.0],
    size: [1.0, 5.0, 4.0],
    color: PIGLIN_SKIN,
}];

pub(super) const ADULT_PIGLIN_HEAD_CHILDREN: [ModelPartDesc; 2] = [
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

pub(super) const ADULT_PIGLIN_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -2.0],
    size: [8.0, 12.0, 4.0],
    color: PIGLIN_SKIN,
}];

pub(super) const ADULT_PIGLIN_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: PIGLIN_SKIN,
}];

pub(super) const ADULT_PIGLIN_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: PIGLIN_SKIN,
}];

pub(super) const ADULT_PIGLIN_RIGHT_SLEEVE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.25, -2.25, -2.25],
    size: [4.5, 12.5, 4.5],
    color: PIGLIN_SKIN,
}];

pub(super) const ADULT_PIGLIN_LEFT_SLEEVE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.25, -2.25, -2.25],
    size: [4.5, 12.5, 4.5],
    color: PIGLIN_SKIN,
}];

pub(super) const ADULT_PIGLIN_RIGHT_ARM_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &ADULT_PIGLIN_RIGHT_SLEEVE,
    children: &[],
}];

pub(super) const ADULT_PIGLIN_LEFT_ARM_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &ADULT_PIGLIN_LEFT_SLEEVE,
    children: &[],
}];

pub(super) const ADULT_PIGLIN_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: PIGLIN_SKIN,
}];

pub(super) const ADULT_PIGLIN_PANTS: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.25, -0.25, -2.25],
    size: [4.5, 12.5, 4.5],
    color: PIGLIN_SKIN,
}];

pub(super) const ADULT_PIGLIN_LEG_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &ADULT_PIGLIN_PANTS,
    children: &[],
}];

// Vanilla 26.1 AdultPiglinModel.createBodyLayer().
pub(super) const ADULT_PIGLIN_PARTS: [ModelPartDesc; 6] = [
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

pub(super) const BABY_PIGLIN_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -3.0, -1.0],
    size: [6.0, 5.0, 3.0],
    color: PIGLIN_SKIN,
}];

pub(super) const BABY_PIGLIN_HEAD: [ModelCubeDesc; 2] = [
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

pub(super) const BABY_PIGLIN_LEFT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.5, -3.0, -2.0],
    size: [1.0, 6.0, 4.0],
    color: PIGLIN_SKIN,
}];

pub(super) const BABY_PIGLIN_RIGHT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.5, -3.0, -2.0],
    size: [1.0, 6.0, 4.0],
    color: PIGLIN_SKIN,
}];

pub(super) const BABY_PIGLIN_HAT_CHILD: ModelPartDesc = ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &[],
    children: &[],
};

pub(super) const BABY_PIGLIN_LEFT_EAR_ROTATED_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [1.0, 1.75, 0.0],
        rotation: [0.0, 0.0, -0.6109],
    },
    cubes: &BABY_PIGLIN_LEFT_EAR,
    children: &[],
}];

pub(super) const BABY_PIGLIN_RIGHT_EAR_ROTATED_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [-1.0, 1.75, 0.0],
        rotation: [0.0, 0.0, 0.6109],
    },
    cubes: &BABY_PIGLIN_RIGHT_EAR,
    children: &[],
}];

pub(super) const BABY_PIGLIN_HEAD_CHILDREN: [ModelPartDesc; 3] = [
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

pub(super) const BABY_PIGLIN_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.5],
    size: [2.0, 5.0, 3.0],
    color: PIGLIN_SKIN,
}];

pub(super) const BABY_PIGLIN_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.5],
    size: [2.0, 5.0, 3.0],
    color: PIGLIN_SKIN,
}];

pub(super) const BABY_PIGLIN_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, 0.0, -1.5],
    size: [3.0, 4.0, 3.0],
    color: PIGLIN_SKIN,
}];

// Vanilla 26.1 BabyPiglinModel.createBodyLayer().
pub(super) const BABY_PIGLIN_PARTS: [ModelPartDesc; 6] = [
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

pub(super) const HOGLIN_HEAD_X_ROT: f32 = 0.87266463;
pub(super) const HOGLIN_EAR_Z_ROT: f32 = std::f32::consts::PI * 2.0 / 9.0;
pub(super) const BABY_HOGLIN_HEAD_X_ROT: f32 = 0.8727;
pub(super) const BABY_HOGLIN_EAR_Z_ROT: f32 = 0.8727;

pub(super) const ADULT_HOGLIN_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-8.0, -7.0, -13.0],
    size: [16.0, 14.0, 26.0],
    color: HOGLIN_RED,
}];

pub(super) const ADULT_HOGLIN_MANE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.001, -0.001, -9.001],
    size: [0.002, 10.002, 19.002],
    color: HOGLIN_RED,
}];

pub(super) const ADULT_HOGLIN_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-7.0, -3.0, -19.0],
    size: [14.0, 6.0, 19.0],
    color: HOGLIN_RED,
}];

pub(super) const ADULT_HOGLIN_RIGHT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-6.0, -1.0, -2.0],
    size: [6.0, 1.0, 4.0],
    color: HOGLIN_RED,
}];

pub(super) const ADULT_HOGLIN_LEFT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, -1.0, -2.0],
    size: [6.0, 1.0, 4.0],
    color: HOGLIN_RED,
}];

pub(super) const ADULT_HOGLIN_HORN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -11.0, -1.0],
    size: [2.0, 11.0, 2.0],
    color: HOGLIN_RED,
}];

pub(super) const ADULT_HOGLIN_FRONT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, 0.0, -3.0],
    size: [6.0, 14.0, 6.0],
    color: HOGLIN_RED,
}];

pub(super) const ADULT_HOGLIN_HIND_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.5, 0.0, -2.5],
    size: [5.0, 11.0, 5.0],
    color: HOGLIN_RED,
}];

pub(super) const ADULT_HOGLIN_BODY_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [0.0, -14.0, -7.0],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &ADULT_HOGLIN_MANE,
    children: &[],
}];

pub(super) const ADULT_HOGLIN_HEAD_CHILDREN: [ModelPartDesc; 4] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [-6.0, -2.0, -3.0],
            rotation: [0.0, 0.0, -HOGLIN_EAR_Z_ROT],
        },
        cubes: &ADULT_HOGLIN_RIGHT_EAR,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [6.0, -2.0, -3.0],
            rotation: [0.0, 0.0, HOGLIN_EAR_Z_ROT],
        },
        cubes: &ADULT_HOGLIN_LEFT_EAR,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-7.0, 2.0, -12.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HOGLIN_HORN,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [7.0, 2.0, -12.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HOGLIN_HORN,
        children: &[],
    },
];

// Vanilla 26.1 ModelLayers.HOGLIN / ZOGLIN: HoglinModel.createBodyLayer().
pub(super) const ADULT_HOGLIN_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 7.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HOGLIN_BODY,
        children: &ADULT_HOGLIN_BODY_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 2.0, -12.0],
            rotation: [HOGLIN_HEAD_X_ROT, 0.0, 0.0],
        },
        cubes: &ADULT_HOGLIN_HEAD,
        children: &ADULT_HOGLIN_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 10.0, -8.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HOGLIN_FRONT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 10.0, -8.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HOGLIN_FRONT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, 13.0, 10.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HOGLIN_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, 13.0, 10.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HOGLIN_HIND_LEG,
        children: &[],
    },
];

pub(super) const BABY_HOGLIN_HEAD: [ModelCubeDesc; 3] = [
    ModelCubeDesc {
        min: [-5.0, -2.2605, -10.547],
        size: [10.0, 4.0, 12.0],
        color: HOGLIN_RED,
    },
    ModelCubeDesc {
        min: [-7.0, -4.0981, -8.4879],
        size: [2.0, 5.0, 2.0],
        color: HOGLIN_RED,
    },
    ModelCubeDesc {
        min: [5.0, -4.0981, -8.4879],
        size: [2.0, 5.0, 2.0],
        color: HOGLIN_RED,
    },
];

pub(super) const BABY_HOGLIN_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-4.02, -14.02, -7.02],
        size: [8.04, 8.04, 14.04],
        color: HOGLIN_RED,
    },
    ModelCubeDesc {
        min: [-0.02, -18.02, -8.02],
        size: [0.04, 6.04, 11.04],
        color: HOGLIN_RED,
    },
];

pub(super) const BABY_HOGLIN_RIGHT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-5.1, -0.5, -2.0],
    size: [6.0, 1.0, 4.0],
    color: HOGLIN_RED,
}];

pub(super) const BABY_HOGLIN_LEFT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.9, -0.5, -2.0],
    size: [6.0, 1.0, 4.0],
    color: HOGLIN_RED,
}];

pub(super) const BABY_HOGLIN_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, 0.0, -1.5],
    size: [3.0, 6.0, 3.0],
    color: HOGLIN_RED,
}];

pub(super) const BABY_HOGLIN_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, -1.0, -1.5],
            rotation: [0.0, 0.0, -BABY_HOGLIN_EAR_Z_ROT],
        },
        cubes: &BABY_HOGLIN_RIGHT_EAR,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, -1.0, -1.5],
            rotation: [0.0, 0.0, BABY_HOGLIN_EAR_Z_ROT],
        },
        cubes: &BABY_HOGLIN_LEFT_EAR,
        children: &[],
    },
];

// Vanilla 26.1 ModelLayers.HOGLIN_BABY / ZOGLIN_BABY:
// BabyHoglinModel.createBodyLayer().
pub(super) const BABY_HOGLIN_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 13.0, -7.0],
            rotation: [BABY_HOGLIN_HEAD_X_ROT, 0.0, 0.0],
        },
        cubes: &BABY_HOGLIN_HEAD,
        children: &BABY_HOGLIN_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 24.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_HOGLIN_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 18.0, 4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_HOGLIN_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, 18.0, 4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_HOGLIN_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 18.0, -4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_HOGLIN_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, 18.0, -4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_HOGLIN_LEG,
        children: &[],
    },
];

pub(super) const RAVAGER_NECK: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-5.0, -1.0, -18.0],
    size: [10.0, 10.0, 18.0],
    color: RAVAGER_GRAY,
}];

pub(super) const RAVAGER_HEAD: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-8.0, -20.0, -14.0],
        size: [16.0, 20.0, 16.0],
        color: RAVAGER_GRAY,
    },
    ModelCubeDesc {
        min: [-2.0, -6.0, -18.0],
        size: [4.0, 8.0, 4.0],
        color: RAVAGER_GRAY,
    },
];

pub(super) const RAVAGER_HORN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, -14.0, -2.0],
    size: [2.0, 14.0, 4.0],
    color: RAVAGER_GRAY,
}];

pub(super) const RAVAGER_MOUTH: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-8.0, 0.0, -16.0],
    size: [16.0, 3.0, 16.0],
    color: RAVAGER_GRAY,
}];

pub(super) const RAVAGER_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-7.0, -10.0, -7.0],
        size: [14.0, 16.0, 20.0],
        color: RAVAGER_GRAY,
    },
    ModelCubeDesc {
        min: [-6.0, 6.0, -7.0],
        size: [12.0, 13.0, 18.0],
        color: RAVAGER_GRAY,
    },
];

pub(super) const RAVAGER_HIND_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -4.0],
    size: [8.0, 37.0, 8.0],
    color: RAVAGER_GRAY,
}];

pub(super) const RAVAGER_FRONT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -4.0],
    size: [8.0, 37.0, 8.0],
    color: RAVAGER_GRAY,
}];

pub(super) const RAVAGER_HEAD_CHILDREN: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [-10.0, -14.0, -8.0],
            rotation: [1.0995574, 0.0, 0.0],
        },
        cubes: &RAVAGER_HORN,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [8.0, -14.0, -8.0],
            rotation: [1.0995574, 0.0, 0.0],
        },
        cubes: &RAVAGER_HORN,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -2.0, 2.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &RAVAGER_MOUTH,
        children: &[],
    },
];

pub(super) const RAVAGER_NECK_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [0.0, 16.0, -17.0],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &RAVAGER_HEAD,
    children: &RAVAGER_HEAD_CHILDREN,
}];

// Vanilla 26.1 ModelLayers.RAVAGER: RavagerModel.createBodyLayer().
pub(super) const RAVAGER_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -7.0, 5.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &RAVAGER_NECK,
        children: &RAVAGER_NECK_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 1.0, 2.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &RAVAGER_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-8.0, -13.0, 18.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &RAVAGER_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [8.0, -13.0, 18.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &RAVAGER_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-8.0, -13.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &RAVAGER_FRONT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [8.0, -13.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &RAVAGER_FRONT_LEG,
        children: &[],
    },
];

pub(super) const ADULT_HORSE_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-5.05, -8.05, -17.05],
    size: [10.1, 10.1, 22.1],
    color: HORSE_BROWN,
}];

pub(super) const ADULT_HORSE_TAIL: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, 0.0, 0.0],
    size: [3.0, 14.0, 4.0],
    color: HORSE_BROWN,
}];

pub(super) const ADULT_HORSE_BODY_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [0.0, -5.0, 2.0],
        rotation: [std::f32::consts::FRAC_PI_6, 0.0, 0.0],
    },
    cubes: &ADULT_HORSE_TAIL,
    children: &[],
}];

pub(super) const ADULT_HORSE_NECK: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.05, -6.0, -2.0],
    size: [4.0, 12.0, 7.0],
    color: HORSE_BROWN,
}];

pub(super) const ADULT_HORSE_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -11.0, -2.0],
    size: [6.0, 5.0, 7.0],
    color: HORSE_BROWN,
}];

pub(super) const ADULT_HORSE_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.551, -12.999, 4.001],
    size: [1.998, 2.998, 0.998],
    color: HORSE_BROWN,
}];

pub(super) const ADULT_HORSE_RIGHT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.549, -12.999, 4.001],
    size: [1.998, 2.998, 0.998],
    color: HORSE_BROWN,
}];

pub(super) const ADULT_HORSE_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_HORSE_EAR,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_HORSE_RIGHT_EAR,
        children: &[],
    },
];

pub(super) const ADULT_HORSE_MANE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -11.0, 5.01],
    size: [2.0, 16.0, 2.0],
    color: HORSE_BROWN,
}];

pub(super) const ADULT_HORSE_UPPER_MOUTH: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -11.0, -7.0],
    size: [4.0, 5.0, 5.0],
    color: HORSE_BROWN,
}];

pub(super) const ADULT_HORSE_HEAD_PARTS_CHILDREN: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_HORSE_HEAD,
        children: &ADULT_HORSE_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_HORSE_MANE,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_HORSE_UPPER_MOUTH,
        children: &[],
    },
];

pub(super) const ADULT_HORSE_LEFT_HIND_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -1.01, -1.0],
    size: [4.0, 11.0, 4.0],
    color: HORSE_BROWN,
}];

pub(super) const ADULT_HORSE_RIGHT_HIND_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -1.01, -1.0],
    size: [4.0, 11.0, 4.0],
    color: HORSE_BROWN,
}];

pub(super) const ADULT_HORSE_LEFT_FRONT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -1.01, -1.9],
    size: [4.0, 11.0, 4.0],
    color: HORSE_BROWN,
}];

pub(super) const ADULT_HORSE_RIGHT_FRONT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -1.01, -1.9],
    size: [4.0, 11.0, 4.0],
    color: HORSE_BROWN,
}];

// Vanilla 26.1 ModelLayers.HORSE:
// AbstractEquineModel.createBodyMesh(CubeDeformation.NONE) with
// LayerDefinitions' MeshTransformer.scaling(1.1F) applied by the emitter.
pub(super) const ADULT_HORSE_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 11.0, 5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_BODY,
        children: &ADULT_HORSE_BODY_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 4.0, -12.0],
            rotation: [std::f32::consts::FRAC_PI_6, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_NECK,
        children: &ADULT_HORSE_HEAD_PARTS_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 14.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_LEFT_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 14.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_RIGHT_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 14.0, -10.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_LEFT_FRONT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 14.0, -10.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_RIGHT_FRONT_LEG,
        children: &[],
    },
];

pub(super) const BABY_HORSE_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -3.5, -7.0],
    size: [8.0, 7.0, 14.0],
    color: HORSE_BROWN,
}];

pub(super) const BABY_HORSE_TAIL: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, -1.5, -1.0],
    size: [3.0, 3.0, 8.0],
    color: HORSE_BROWN,
}];

pub(super) const BABY_HORSE_BODY_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [0.0, -1.0, 7.0],
        rotation: [-0.7418, 0.0, 0.0],
    },
    cubes: &BABY_HORSE_TAIL,
    children: &[],
}];

pub(super) const BABY_HORSE_LEFT_HIND_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, -1.0, -1.5],
    size: [3.0, 9.0, 3.0],
    color: HORSE_BROWN,
}];

pub(super) const BABY_HORSE_RIGHT_HIND_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, -1.0, -1.5],
    size: [3.0, 9.0, 3.0],
    color: HORSE_BROWN,
}];

pub(super) const BABY_HORSE_LEFT_FRONT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, -1.0, -1.5],
    size: [3.0, 9.0, 3.0],
    color: HORSE_BROWN,
}];

pub(super) const BABY_HORSE_RIGHT_FRONT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, -1.0, -1.5],
    size: [3.0, 9.0, 3.0],
    color: HORSE_BROWN,
}];

pub(super) const BABY_HORSE_NECK: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -6.0, -2.0],
    size: [4.0, 8.0, 4.0],
    color: HORSE_BROWN,
}];

pub(super) const BABY_HORSE_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -3.9484, -6.705],
    size: [6.0, 4.0, 9.0],
    color: HORSE_BROWN,
}];

pub(super) const BABY_HORSE_LEFT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.5, -0.8],
    size: [2.0, 3.0, 1.0],
    color: HORSE_BROWN,
}];

pub(super) const BABY_HORSE_RIGHT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.5, -0.5],
    size: [2.0, 3.0, 1.0],
    color: HORSE_BROWN,
}];

pub(super) const BABY_HORSE_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, -4.2484, 1.9451],
            rotation: [0.0, 0.0, 0.2618],
        },
        cubes: &BABY_HORSE_LEFT_EAR,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, -4.2484, 1.645],
            rotation: [0.0, 0.0, -0.2618],
        },
        cubes: &BABY_HORSE_RIGHT_EAR,
        children: &[],
    },
];

pub(super) const BABY_HORSE_HEAD_PARTS_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [0.0, -6.0516, -0.2951],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &BABY_HORSE_HEAD,
    children: &BABY_HORSE_HEAD_CHILDREN,
}];

// Vanilla 26.1 ModelLayers.HORSE_BABY:
// BabyHorseModel.createBabyMesh(CubeDeformation.NONE), without livingHorseScale.
pub(super) const BABY_HORSE_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 12.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_HORSE_BODY,
        children: &BABY_HORSE_BODY_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.4, 16.0, 5.4],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_HORSE_LEFT_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.4, 16.0, 5.4],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_HORSE_RIGHT_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.4, 16.0, -5.4],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_HORSE_LEFT_FRONT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.4, 16.0, -5.4],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_HORSE_RIGHT_FRONT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 10.0, -6.0],
            rotation: [0.6109, 0.0, 0.0],
        },
        cubes: &BABY_HORSE_NECK,
        children: &BABY_HORSE_HEAD_PARTS_CHILDREN,
    },
];

pub(super) const ADULT_DONKEY_CHEST: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -2.0],
    size: [8.0, 8.0, 3.0],
    color: DONKEY_GRAY,
}];

pub(super) const ADULT_DONKEY_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -7.0, 0.0],
    size: [2.0, 7.0, 1.0],
    color: DONKEY_GRAY,
}];

pub(super) const ADULT_DONKEY_BODY_CHILDREN_WITH_CHEST: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -5.0, 2.0],
            rotation: [std::f32::consts::FRAC_PI_6, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_TAIL,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [6.0, -8.0, 0.0],
            rotation: [0.0, -std::f32::consts::FRAC_PI_2, 0.0],
        },
        cubes: &ADULT_DONKEY_CHEST,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-6.0, -8.0, 0.0],
            rotation: [0.0, std::f32::consts::FRAC_PI_2, 0.0],
        },
        cubes: &ADULT_DONKEY_CHEST,
        children: &[],
    },
];

pub(super) const ADULT_DONKEY_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [1.25, -10.0, 4.0],
            rotation: [0.2617994, 0.0, 0.2617994],
        },
        cubes: &ADULT_DONKEY_EAR,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.25, -10.0, 4.0],
            rotation: [0.2617994, 0.0, -0.2617994],
        },
        cubes: &ADULT_DONKEY_EAR,
        children: &[],
    },
];

pub(super) const ADULT_DONKEY_HEAD_PARTS_CHILDREN: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_HORSE_HEAD,
        children: &ADULT_DONKEY_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_HORSE_MANE,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_HORSE_UPPER_MOUTH,
        children: &[],
    },
];

// Vanilla 26.1 ModelLayers.DONKEY and ModelLayers.MULE:
// AbstractEquineModel.createBodyMesh(CubeDeformation.NONE), DonkeyModel.modifyMesh(),
// then MeshTransformer.scaling(0.87F or 0.92F) applied by the emitter.
pub(super) const ADULT_DONKEY_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 11.0, 5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_BODY,
        children: &ADULT_HORSE_BODY_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 4.0, -12.0],
            rotation: [std::f32::consts::FRAC_PI_6, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_NECK,
        children: &ADULT_DONKEY_HEAD_PARTS_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 14.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_LEFT_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 14.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_RIGHT_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 14.0, -10.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_LEFT_FRONT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 14.0, -10.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_RIGHT_FRONT_LEG,
        children: &[],
    },
];

pub(super) const ADULT_DONKEY_PARTS_WITH_CHEST: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 11.0, 5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_BODY,
        children: &ADULT_DONKEY_BODY_CHILDREN_WITH_CHEST,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 4.0, -12.0],
            rotation: [std::f32::consts::FRAC_PI_6, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_NECK,
        children: &ADULT_DONKEY_HEAD_PARTS_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 14.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_LEFT_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 14.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_RIGHT_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 14.0, -10.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_LEFT_FRONT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 14.0, -10.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_HORSE_RIGHT_FRONT_LEG,
        children: &[],
    },
];

pub(super) const BABY_DONKEY_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-5.0, -3.0, -7.0],
    size: [8.0, 6.0, 14.0],
    color: DONKEY_GRAY,
}];

pub(super) const BABY_DONKEY_TAIL_R1: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.5, -1.0, -0.5],
    size: [3.0, 3.0, 8.0],
    color: DONKEY_GRAY,
}];

pub(super) const BABY_DONKEY_TAIL_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [0.0, 0.0, 0.0],
        rotation: [-0.7418, 0.0, 0.0],
    },
    cubes: &BABY_DONKEY_TAIL_R1,
    children: &[],
}];

pub(super) const BABY_DONKEY_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.5, -1.5, -1.5],
    size: [3.0, 8.0, 3.0],
    color: DONKEY_GRAY,
}];

pub(super) const BABY_DONKEY_NECK_R1: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -6.0, -3.0],
    size: [4.0, 8.0, 4.0],
    color: DONKEY_GRAY,
}];

pub(super) const BABY_DONKEY_HEAD_R1: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -3.6, -8.4],
    size: [6.0, 4.0, 9.0],
    color: DONKEY_GRAY,
}];

pub(super) const BABY_DONKEY_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -6.5, -0.3],
    size: [2.0, 7.0, 1.0],
    color: DONKEY_GRAY,
}];

pub(super) const BABY_DONKEY_HEAD_CHILDREN: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -1.0, 1.0],
            rotation: [0.3927, 0.0, 0.0],
        },
        cubes: &BABY_DONKEY_HEAD_R1,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, -3.5, -1.0],
            rotation: [0.48, 0.0, 0.48],
        },
        cubes: &BABY_DONKEY_EAR,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, -3.5, -1.0],
            rotation: [0.48, 0.0, -0.48],
        },
        cubes: &BABY_DONKEY_EAR,
        children: &[],
    },
];

pub(super) const BABY_DONKEY_HEAD_PARTS_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 0.0, 0.0],
            rotation: [0.3927, 0.0, 0.0],
        },
        cubes: &BABY_DONKEY_NECK_R1,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -5.0, -3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &[],
        children: &BABY_DONKEY_HEAD_CHILDREN,
    },
];

pub(super) const BABY_DONKEY_BODY_CHILDREN: [ModelPartDesc; 8] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -1.5, 6.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &[],
        children: &BABY_DONKEY_TAIL_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.25, 3.5, 5.25],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_DONKEY_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.4, 3.5, 5.4],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_DONKEY_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.4, 3.5, -5.3],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_DONKEY_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.4, 3.5, -5.4],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_DONKEY_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -3.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &[],
        children: &BABY_DONKEY_HEAD_PARTS_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, 10.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &[],
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, 10.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &[],
        children: &[],
    },
];

// Vanilla 26.1 ModelLayers.DONKEY_BABY and ModelLayers.MULE_BABY:
// BabyDonkeyModel.createBabyLayer(); both families share geometry and differ by texture.
pub(super) const BABY_DONKEY_PARTS: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [1.0, 14.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &BABY_DONKEY_BODY,
    children: &BABY_DONKEY_BODY_CHILDREN,
}];

pub(super) const ADULT_CAMEL_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-7.5, -12.0, -23.5],
    size: [15.0, 12.0, 27.0],
    color: CAMEL_TAN,
}];

pub(super) const ADULT_CAMEL_HUMP: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.5, -5.0, -5.5],
    size: [9.0, 5.0, 11.0],
    color: CAMEL_TAN,
}];

pub(super) const ADULT_CAMEL_TAIL: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, 0.0, 0.0],
    size: [3.0, 14.0, 0.0],
    color: CAMEL_TAN,
}];

pub(super) const ADULT_CAMEL_HEAD: [ModelCubeDesc; 3] = [
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

pub(super) const ADULT_CAMEL_LEFT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.5, 0.5, -1.0],
    size: [3.0, 1.0, 2.0],
    color: CAMEL_TAN,
}];

pub(super) const ADULT_CAMEL_RIGHT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.5, 0.5, -1.0],
    size: [3.0, 1.0, 2.0],
    color: CAMEL_TAN,
}];

pub(super) const ADULT_CAMEL_LEFT_HIND_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.5, 2.0, -2.5],
    size: [5.0, 21.0, 5.0],
    color: CAMEL_TAN,
}];

pub(super) const ADULT_CAMEL_RIGHT_HIND_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.5, 2.0, -2.5],
    size: [5.0, 21.0, 5.0],
    color: CAMEL_TAN,
}];

pub(super) const ADULT_CAMEL_LEFT_FRONT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.5, 2.0, -2.5],
    size: [5.0, 21.0, 5.0],
    color: CAMEL_TAN,
}];

pub(super) const ADULT_CAMEL_RIGHT_FRONT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.5, 2.0, -2.5],
    size: [5.0, 21.0, 5.0],
    color: CAMEL_TAN,
}];

pub(super) const ADULT_CAMEL_HEAD_CHILDREN: [ModelPartDesc; 2] = [
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

pub(super) const ADULT_CAMEL_BODY_CHILDREN: [ModelPartDesc; 3] = [
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
pub(super) const ADULT_CAMEL_PARTS: [ModelPartDesc; 5] = [
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

pub(super) const BABY_CAMEL_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.5, -4.0, -8.0],
    size: [9.0, 8.0, 16.0],
    color: CAMEL_TAN,
}];

pub(super) const BABY_CAMEL_TAIL: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, -0.5, 0.0],
    size: [3.0, 9.0, 0.0],
    color: CAMEL_TAN,
}];

pub(super) const BABY_CAMEL_HEAD: [ModelCubeDesc; 3] = [
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

pub(super) const BABY_CAMEL_RIGHT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -0.5, -1.0],
    size: [3.0, 1.0, 2.0],
    color: CAMEL_TAN,
}];

pub(super) const BABY_CAMEL_LEFT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, -0.5, -1.0],
    size: [3.0, 1.0, 2.0],
    color: CAMEL_TAN,
}];

pub(super) const BABY_CAMEL_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, -0.5, -1.5],
    size: [3.0, 13.0, 3.0],
    color: CAMEL_TAN,
}];

pub(super) const BABY_CAMEL_HEAD_CHILDREN: [ModelPartDesc; 2] = [
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

pub(super) const BABY_CAMEL_BODY_CHILDREN: [ModelPartDesc; 2] = [
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
pub(super) const BABY_CAMEL_PARTS: [ModelPartDesc; 5] = [
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

pub(super) const ADULT_LLAMA_HEAD: [ModelCubeDesc; 4] = [
    ModelCubeDesc {
        min: [-2.0, -14.0, -10.0],
        size: [4.0, 4.0, 9.0],
        color: LLAMA_CREAMY,
    },
    ModelCubeDesc {
        min: [-4.0, -16.0, -6.0],
        size: [8.0, 18.0, 6.0],
        color: LLAMA_CREAMY,
    },
    ModelCubeDesc {
        min: [-4.0, -19.0, -4.0],
        size: [3.0, 3.0, 2.0],
        color: LLAMA_CREAMY,
    },
    ModelCubeDesc {
        min: [1.0, -19.0, -4.0],
        size: [3.0, 3.0, 2.0],
        color: LLAMA_CREAMY,
    },
];

pub(super) const ADULT_LLAMA_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-6.0, -10.0, -7.0],
    size: [12.0, 18.0, 10.0],
    color: LLAMA_CREAMY,
}];

pub(super) const LLAMA_CHEST: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, 0.0, 0.0],
    size: [8.0, 8.0, 3.0],
    color: LLAMA_CREAMY,
}];

pub(super) const ADULT_LLAMA_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 14.0, 4.0],
    color: LLAMA_CREAMY,
}];

pub(super) const ADULT_LLAMA_RIGHT_CHEST_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [-8.5, 3.0, 3.0],
        rotation: [0.0, std::f32::consts::FRAC_PI_2, 0.0],
    },
    cubes: &LLAMA_CHEST,
    children: &[],
};

pub(super) const ADULT_LLAMA_LEFT_CHEST_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [5.5, 3.0, 3.0],
        rotation: [0.0, std::f32::consts::FRAC_PI_2, 0.0],
    },
    cubes: &LLAMA_CHEST,
    children: &[],
};

// Vanilla 26.1 ModelLayers.LLAMA / TRADER_LLAMA:
// LlamaModel.createBodyLayer(CubeDeformation.NONE). Chest parts are only visible
// when LlamaRenderState.hasChest is true.
pub(super) const ADULT_LLAMA_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 7.0, -6.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_LLAMA_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 5.0, 2.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &ADULT_LLAMA_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.5, 10.0, 6.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_LLAMA_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.5, 10.0, 6.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_LLAMA_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.5, 10.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_LLAMA_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.5, 10.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_LLAMA_LEG,
        children: &[],
    },
];

pub(super) const ADULT_LLAMA_PARTS_WITH_CHEST: [ModelPartDesc; 8] = [
    ADULT_LLAMA_PARTS[0],
    ADULT_LLAMA_PARTS[1],
    ADULT_LLAMA_RIGHT_CHEST_PART,
    ADULT_LLAMA_LEFT_CHEST_PART,
    ADULT_LLAMA_PARTS[2],
    ADULT_LLAMA_PARTS[3],
    ADULT_LLAMA_PARTS[4],
    ADULT_LLAMA_PARTS[5],
];

pub(super) const BABY_LLAMA_HEAD: [ModelCubeDesc; 4] = [
    ModelCubeDesc {
        min: [-3.0, -9.0, -4.0],
        size: [6.0, 11.0, 4.0],
        color: LLAMA_CREAMY,
    },
    ModelCubeDesc {
        min: [-1.5, -7.0, -7.0],
        size: [3.0, 3.0, 3.0],
        color: LLAMA_CREAMY,
    },
    ModelCubeDesc {
        min: [0.5, -11.0, -3.0],
        size: [2.0, 2.0, 2.0],
        color: LLAMA_CREAMY,
    },
    ModelCubeDesc {
        min: [-2.5, -11.0, -3.0],
        size: [2.0, 2.0, 2.0],
        color: LLAMA_CREAMY,
    },
];

pub(super) const BABY_LLAMA_RIGHT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.4, -0.5, -1.5],
    size: [3.0, 8.0, 3.0],
    color: LLAMA_CREAMY,
}];

pub(super) const BABY_LLAMA_LEFT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.6, -0.5, -1.5],
    size: [3.0, 8.0, 3.0],
    color: LLAMA_CREAMY,
}];

pub(super) const BABY_LLAMA_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -3.0, -8.5],
    size: [8.0, 6.0, 13.0],
    color: LLAMA_CREAMY,
}];

// Vanilla 26.1 ModelLayers.LLAMA_BABY / TRADER_LLAMA_BABY:
// BabyLlamaModel.createBodyLayer(CubeDeformation.NONE). The layer includes
// chest parts, but LlamaRenderer sets hasChest=false for babies.
pub(super) const BABY_LLAMA_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 12.0, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_LLAMA_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 16.5, 4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_LLAMA_RIGHT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, 16.5, 4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_LLAMA_LEFT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 16.5, -3.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_LLAMA_RIGHT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, 16.5, -3.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_LLAMA_LEFT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 14.0, 2.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_LLAMA_BODY,
        children: &[],
    },
];

pub(super) const ADULT_GOAT_HEAD: [ModelCubeDesc; 3] = [
    ModelCubeDesc {
        min: [-6.0, -11.0, -10.0],
        size: [3.0, 2.0, 1.0],
        color: GOAT_WHITE,
    },
    ModelCubeDesc {
        min: [2.0, -11.0, -10.0],
        size: [3.0, 2.0, 1.0],
        color: GOAT_WHITE,
    },
    ModelCubeDesc {
        min: [-0.5, -3.0, -14.0],
        size: [0.0, 7.0, 5.0],
        color: GOAT_BEARD,
    },
];

pub(super) const ADULT_GOAT_LEFT_HORN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.01, -16.0, -10.0],
    size: [2.0, 7.0, 2.0],
    color: GOAT_HORN,
}];

pub(super) const ADULT_GOAT_RIGHT_HORN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.99, -16.0, -10.0],
    size: [2.0, 7.0, 2.0],
    color: GOAT_HORN,
}];

pub(super) const ADULT_GOAT_NOSE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -4.0, -8.0],
    size: [5.0, 7.0, 10.0],
    color: GOAT_WHITE,
}];

pub(super) const ADULT_GOAT_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-4.0, -17.0, -7.0],
        size: [9.0, 11.0, 16.0],
        color: GOAT_WHITE,
    },
    ModelCubeDesc {
        min: [-5.0, -18.0, -8.0],
        size: [11.0, 14.0, 11.0],
        color: GOAT_WHITE,
    },
];

pub(super) const ADULT_GOAT_HIND_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 4.0, 0.0],
    size: [3.0, 6.0, 3.0],
    color: GOAT_WHITE,
}];

pub(super) const ADULT_GOAT_FRONT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, 0.0],
    size: [3.0, 10.0, 3.0],
    color: GOAT_WHITE,
}];

pub(super) const ADULT_GOAT_LEFT_HORN_PART: ModelPartDesc = ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &ADULT_GOAT_LEFT_HORN,
    children: &[],
};

pub(super) const ADULT_GOAT_RIGHT_HORN_PART: ModelPartDesc = ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &ADULT_GOAT_RIGHT_HORN,
    children: &[],
};

pub(super) const ADULT_GOAT_NOSE_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [0.0, -8.0, -8.0],
        rotation: [0.9599, 0.0, 0.0],
    },
    cubes: &ADULT_GOAT_NOSE,
    children: &[],
};

pub(super) const ADULT_GOAT_HEAD_CHILDREN: [ModelPartDesc; 3] = [
    ADULT_GOAT_LEFT_HORN_PART,
    ADULT_GOAT_RIGHT_HORN_PART,
    ADULT_GOAT_NOSE_PART,
];

pub(super) const ADULT_GOAT_HEAD_INDEX: usize = 0;
pub(super) const ADULT_GOAT_LEFT_HORN_CHILD_INDEX: usize = 0;
pub(super) const ADULT_GOAT_RIGHT_HORN_CHILD_INDEX: usize = 1;

// Vanilla 26.1 ModelLayers.GOAT: GoatModel.createBodyLayer().
pub(super) const ADULT_GOAT_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [1.0, 14.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_GOAT_HEAD,
        children: &ADULT_GOAT_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 24.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_GOAT_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.0, 14.0, 4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_GOAT_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 14.0, 4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_GOAT_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.0, 14.0, -6.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_GOAT_FRONT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 14.0, -6.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_GOAT_FRONT_LEG,
        children: &[],
    },
];

pub(super) const BABY_GOAT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -0.5, -1.0],
    size: [2.0, 5.0, 2.0],
    color: GOAT_WHITE,
}];

pub(super) const BABY_GOAT_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-3.0, -2.3, -4.5],
        size: [6.0, 5.0, 9.0],
        color: GOAT_WHITE,
    },
    ModelCubeDesc {
        min: [-2.5, -2.2, -4.0],
        size: [5.0, 4.0, 8.0],
        color: GOAT_WHITE,
    },
];

pub(super) const BABY_GOAT_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -3.8126, -5.1548],
    size: [4.0, 4.0, 6.0],
    color: GOAT_WHITE,
}];

pub(super) const BABY_GOAT_RIGHT_HORN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, -4.5, 0.0],
    size: [1.0, 2.0, 1.0],
    color: GOAT_HORN,
}];

pub(super) const BABY_GOAT_LEFT_HORN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [2.0, -4.5, 0.0],
    size: [1.0, 2.0, 1.0],
    color: GOAT_HORN,
}];

pub(super) const BABY_GOAT_RIGHT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -0.5, -0.5],
    size: [2.0, 1.0, 1.0],
    color: GOAT_WHITE,
}];

pub(super) const BABY_GOAT_LEFT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, -0.5, -0.5],
    size: [2.0, 1.0, 1.0],
    color: GOAT_WHITE,
}];

pub(super) const BABY_GOAT_HEAD_MAIN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -2.5, -4.0],
    size: [4.0, 4.0, 6.0],
    color: GOAT_WHITE,
}];

pub(super) const BABY_GOAT_RIGHT_HORN_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [-1.5, -1.5, -1.0],
        rotation: [-0.3926991, 0.0, 0.0],
    },
    cubes: &BABY_GOAT_RIGHT_HORN,
    children: &[],
};

pub(super) const BABY_GOAT_LEFT_HORN_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [-1.5, -1.5, -1.0],
        rotation: [-0.3926991, 0.0, 0.0],
    },
    cubes: &BABY_GOAT_LEFT_HORN,
    children: &[],
};

pub(super) const BABY_GOAT_RIGHT_EAR_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [-1.7, -2.3126, 0.1452],
        rotation: [0.0, -0.5236, 0.0],
    },
    cubes: &BABY_GOAT_RIGHT_EAR,
    children: &[],
};

pub(super) const BABY_GOAT_LEFT_EAR_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [1.7, -2.3126, 0.1452],
        rotation: [0.0, 0.5236, 0.0],
    },
    cubes: &BABY_GOAT_LEFT_EAR,
    children: &[],
};

pub(super) const BABY_GOAT_HEAD_MAIN_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [0.0, -1.3126, -1.1548],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &BABY_GOAT_HEAD_MAIN,
    children: &[],
};

pub(super) const BABY_GOAT_HEAD_CHILDREN: [ModelPartDesc; 5] = [
    BABY_GOAT_RIGHT_HORN_PART,
    BABY_GOAT_LEFT_HORN_PART,
    BABY_GOAT_RIGHT_EAR_PART,
    BABY_GOAT_LEFT_EAR_PART,
    BABY_GOAT_HEAD_MAIN_PART,
];

pub(super) const BABY_GOAT_HEAD_INDEX: usize = 5;
pub(super) const BABY_GOAT_LEFT_HORN_CHILD_INDEX: usize = 1;
pub(super) const BABY_GOAT_RIGHT_HORN_CHILD_INDEX: usize = 0;

// Vanilla 26.1 ModelLayers.GOAT_BABY: BabyGoatModel.createBodyLayer().
pub(super) const BABY_GOAT_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [1.5, 19.5, 3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_GOAT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.5, 19.5, 3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_GOAT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.5, 19.5, -2.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_GOAT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.5, 19.5, -2.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_GOAT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 17.8, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_GOAT_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 15.5, -3.0],
            rotation: [0.4363, 0.0, 0.0],
        },
        cubes: &BABY_GOAT_HEAD,
        children: &BABY_GOAT_HEAD_CHILDREN,
    },
];

pub(super) const ADULT_POLAR_BEAR_HEAD: [ModelCubeDesc; 4] = [
    ModelCubeDesc {
        min: [-3.5, -3.0, -3.0],
        size: [7.0, 7.0, 7.0],
        color: POLAR_BEAR_WHITE,
    },
    ModelCubeDesc {
        min: [-2.5, 1.0, -6.0],
        size: [5.0, 3.0, 3.0],
        color: POLAR_BEAR_WHITE,
    },
    ModelCubeDesc {
        min: [-4.5, -4.0, -1.0],
        size: [2.0, 2.0, 1.0],
        color: POLAR_BEAR_WHITE,
    },
    ModelCubeDesc {
        min: [2.5, -4.0, -1.0],
        size: [2.0, 2.0, 1.0],
        color: POLAR_BEAR_WHITE,
    },
];

pub(super) const ADULT_POLAR_BEAR_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-5.0, -13.0, -7.0],
        size: [14.0, 14.0, 11.0],
        color: POLAR_BEAR_WHITE,
    },
    ModelCubeDesc {
        min: [-4.0, -25.0, -7.0],
        size: [12.0, 12.0, 10.0],
        color: POLAR_BEAR_WHITE,
    },
];

pub(super) const ADULT_POLAR_BEAR_HIND_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 10.0, 8.0],
    color: POLAR_BEAR_WHITE,
}];

pub(super) const ADULT_POLAR_BEAR_FRONT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 10.0, 6.0],
    color: POLAR_BEAR_WHITE,
}];

// Vanilla 26.1 ModelLayers.POLAR_BEAR: PolarBearModel.createBodyLayer()
// with LayerDefinition MeshTransformer.scaling(1.2F) applied at emission.
pub(super) const ADULT_POLAR_BEAR_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 10.0, -16.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_POLAR_BEAR_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 9.0, 12.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &ADULT_POLAR_BEAR_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.5, 14.0, 6.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_POLAR_BEAR_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.5, 14.0, 6.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_POLAR_BEAR_HIND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.5, 14.0, -8.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_POLAR_BEAR_FRONT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.5, 14.0, -8.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_POLAR_BEAR_FRONT_LEG,
        children: &[],
    },
];

pub(super) const BABY_POLAR_BEAR_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -3.5, -6.0],
    size: [8.0, 7.0, 12.0],
    color: POLAR_BEAR_WHITE,
}];

pub(super) const BABY_POLAR_BEAR_HEAD: [ModelCubeDesc; 4] = [
    ModelCubeDesc {
        min: [-3.0, -2.625, -4.25],
        size: [6.0, 5.0, 4.0],
        color: POLAR_BEAR_WHITE,
    },
    ModelCubeDesc {
        min: [-2.0, 0.375, -6.25],
        size: [4.0, 2.0, 2.0],
        color: POLAR_BEAR_WHITE,
    },
    ModelCubeDesc {
        min: [-4.0, -3.625, -2.75],
        size: [2.0, 2.0, 1.0],
        color: POLAR_BEAR_WHITE,
    },
    ModelCubeDesc {
        min: [2.0, -3.625, -2.75],
        size: [2.0, 2.0, 1.0],
        color: POLAR_BEAR_WHITE,
    },
];

pub(super) const BABY_POLAR_BEAR_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, -0.5, -1.5],
    size: [3.0, 3.0, 3.0],
    color: POLAR_BEAR_WHITE,
}];

// Vanilla 26.1 ModelLayers.POLAR_BEAR_BABY: BabyPolarBearModel.createBodyLayer().
pub(super) const BABY_POLAR_BEAR_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 17.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_POLAR_BEAR_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 18.625, -5.75],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_POLAR_BEAR_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 21.5, 4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_POLAR_BEAR_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, 21.5, 4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_POLAR_BEAR_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 21.5, -4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_POLAR_BEAR_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, 21.5, -4.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_POLAR_BEAR_LEG,
        children: &[],
    },
];

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
